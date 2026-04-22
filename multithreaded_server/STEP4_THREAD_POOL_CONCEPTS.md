# Step 4 — What is a Thread Pool?

> *"Like Python's `ThreadPoolExecutor` — but we'll build it from scratch."*

---

## 🐍 You Already Know This: `ThreadPoolExecutor`

```python
from concurrent.futures import ThreadPoolExecutor

# Create a pool of exactly 4 workers — no more, ever
with ThreadPoolExecutor(max_workers=4) as pool:
    for request in incoming_requests:
        pool.submit(handle_connection, request)
        # ☝️ If all 4 workers are busy, this WAITS until one is free
        #    It does NOT spawn a new thread — it reuses existing ones
```

A thread pool is simply:
> **A fixed set of pre-created threads that wait for work, do it, then wait again.**

---

## 🆚 Naive Threads vs Thread Pool

| | Naive (Step 3) | Thread Pool (Step 5) |
| :--- | :--- | :--- |
| **Threads created** | 1 per request (unlimited) | Fixed number upfront |
| **Memory** | Grows forever | Capped |
| **Under attack** | Server crashes | Extra requests wait in queue |
| **Python equivalent** | `threading.Thread()` per request | `ThreadPoolExecutor` |
| **Production ready?** | ❌ | ✅ |

---

## 🏗️ The Architecture We're Building

```
MAIN THREAD                         THREAD POOL (4 workers)
───────────                         ──────────────────────────────────────
                                    Worker 1: 😴 waiting...
                                    Worker 2: 😴 waiting...
                                    Worker 3: 😴 waiting...
                                    Worker 4: 😴 waiting...

Accepts request 1 ─────────────────► Worker 1: 🔨 handling request 1
Accepts request 2 ─────────────────► Worker 2: 🔨 handling request 2
Accepts request 3 ─────────────────► Worker 3: 🔨 handling request 3
Accepts request 4 ─────────────────► Worker 4: 🔨 handling request 4
Accepts request 5 ──[waits]────────► (queued — all workers busy)
                                    Worker 1: ✅ done → 🔨 picks up request 5
```

The main thread just sends jobs down a **channel**. Workers pick them up whenever they are free.

---

## 🧱 The 3 Building Blocks

Our thread pool has 3 parts. We'll build each one in Step 5.

```
┌──────────────────────────────────────────────────────┐
│                     ThreadPool                       │
│                                                      │
│   pool.execute(job)                                  │
│         │                                            │
│         ▼                                            │
│      sender ──────────────────► mpsc::Channel        │
│                                       │              │
│                          ┌────────────┘              │
│                          │  Arc<Mutex<Receiver>>     │
│                          │  (shared between workers) │
│                          ├──────────► Worker 1 🧵    │
│                          ├──────────► Worker 2 🧵    │
│                          ├──────────► Worker 3 🧵    │
│                          └──────────► Worker 4 🧵    │
└──────────────────────────────────────────────────────┘
```

| Part | Rust Type | Python Equivalent | Role |
| :--- | :--- | :--- | :--- |
| **ThreadPool** | `struct ThreadPool` | `ThreadPoolExecutor` | Public API — accepts jobs via `execute()` |
| **Worker** | `struct Worker` | Internal thread in the pool | A thread that loops forever, waiting for jobs |
| **Channel** | `mpsc::channel` | `queue.Queue()` | The pipe that carries jobs from pool to workers |

---

## 🔑 Three New Rust Concepts You Need First

### 1. `mpsc::channel` — The Job Queue / Message Pipe

`mpsc` = **M**ultiple **P**roducer, **S**ingle **C**onsumer

```python
# Python equivalent
import queue

job_queue = queue.Queue()

# Producer (main thread) sends jobs
job_queue.put(lambda: handle_connection(stream))

# Consumer (worker thread) receives and runs jobs
job = job_queue.get()  # blocks until a job is available
job()
```

```rust
// Rust equivalent
use std::sync::mpsc;

let (sender, receiver) = mpsc::channel::<Job>();

// Producer (main thread) sends jobs
sender.send(job).unwrap();

// Consumer (worker thread) receives and runs jobs
let job = receiver.recv().unwrap(); // blocks until a job is available
job();
```

Key point: `receiver.recv()` **blocks** the worker thread until a job arrives —
exactly like Python's `queue.Queue.get()`. The worker thread doesn't burn CPU
while waiting; the OS puts it to sleep until woken up.

---

### 2. `Arc<Mutex<T>>` — Shared, Safe, Mutable Data Across Threads

This is the most important new concept. In Python you can share a queue between
threads for free. In Rust, you must be **explicit** about shared mutable access.

```python
# Python — shared queue between threads (GIL protects it implicitly)
job_queue = queue.Queue()  # all workers share this reference safely
```

```rust
// Rust — you must wrap the receiver in Arc<Mutex<...>>
use std::sync::{Arc, Mutex};

let receiver = Arc::new(Mutex::new(receiver));
//             ─┬─                            ── Arc  = shared ownership
//               └─────────────────────────── ── Mutex = exclusive lock
```

#### `Arc` — Atomic Reference Count (shared ownership)

```python
# Python reference counting is automatic
receiver = create_receiver()
worker1_ref = receiver  # both point to the same object, refcount = 2
worker2_ref = receiver  # refcount = 3
```

```rust
// Rust requires explicit Arc for shared ownership across threads
let shared = Arc::new(receiver);         // refcount = 1
let worker1_ref = Arc::clone(&shared);   // refcount = 2  (cheap clone, just increments counter)
let worker2_ref = Arc::clone(&shared);   // refcount = 3
// dropped when last Arc goes out of scope
```

> `Arc` = like Python's object reference counting, but **thread-safe**
> (uses atomic CPU operations instead of a GIL).

#### `Mutex` — Mutual Exclusion (only one thread at a time)

```python
# Python — queue.Queue is already thread-safe internally
job = job_queue.get()  # Queue handles the locking for you
```

```rust
// Rust — you lock the Mutex explicitly
let job = receiver        // the Arc<Mutex<Receiver>>
    .lock()               // acquire the lock — blocks if another thread holds it
    .unwrap()             // unwrap in case the lock is poisoned (a thread panicked)
    .recv()               // now safely call recv() on the inner Receiver
    .unwrap();
```

The pattern `arc.lock().unwrap().do_something()` will appear often in Step 5.

#### Putting it together — why both?

| Problem | Solution |
| :--- | :--- |
| Multiple workers need to **own** the receiver | `Arc` — shared ownership via reference counting |
| Only one worker should **read** from the receiver at a time | `Mutex` — exclusive lock |

```
Worker 1: arc.lock() → ✅ got lock → recv() → picks up job → releases lock
Worker 2: arc.lock() → ⏳ waiting  → lock released → recv() → picks up next job
Worker 3: arc.lock() → ⏳ waiting  → ...
```

---

### 3. `Box<dyn FnOnce() + Send + 'static>` — Storing Any Function as a Value

In Python, functions are just objects. You can put them in a list or queue effortlessly:

```python
job_queue = queue.Queue()
job_queue.put(lambda: handle_connection(stream))  # easy — functions are objects
```

In Rust, to store "any callable" on the heap you need a **trait object**:

```rust
type Job = Box<dyn FnOnce() + Send + 'static>;
//          ─┬─  ────┬────   ──┬─   ────┬───
//           │       │         │         └── must be valid for thread's entire lifetime
//           │       │         └──────────── can be safely sent across thread boundary
//           │       └────────────────────── any type that implements FnOnce (callable once)
//           └────────────────────────────── heap-allocated, fixed size pointer
```

Breaking it down:

| Part | Meaning | Python equivalent |
| :--- | :--- | :--- |
| `Box<...>` | Heap-allocated pointer of fixed size | Just a reference |
| `dyn FnOnce()` | Any type that can be called once | Any `callable` |
| `Send` | Safe to transfer ownership to another thread | N/A (GIL handles this) |
| `'static` | Contains no short-lived borrows | N/A |

```rust
// We create a Job like this:
let job: Job = Box::new(move || {
    handle_connection(stream); // captures 'stream' by move
});

// We run it like this:
job(); // calls the closure once
```

---

## 🗺️ The Full Plan for Step 5

Here's exactly what we'll build, piece by piece:

```rust
// 1. The Job type alias
type Job = Box<dyn FnOnce() + Send + 'static>;

// 2. The Worker struct — wraps a thread that loops waiting for jobs
struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

// 3. The ThreadPool struct — holds workers + the sender end of the channel
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

// 4. ThreadPool::new(size) — creates the channel, spawns workers
impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool { ... }
    pub fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static { ... }
}
```

---

## ✅ Summary of Step 4

- ✅ A **Thread Pool** = fixed pre-created workers + a job queue — no unlimited spawning
- ✅ `mpsc::channel` = the job queue (like Python's `queue.Queue`) — workers block on `recv()`
- ✅ `Arc` = shared ownership across threads (like Python's reference counting, but explicit and thread-safe)
- ✅ `Mutex` = only one worker grabs a job at a time (exclusive lock)
- ✅ `Arc<Mutex<T>>` = the combination needed to safely share the receiver between all workers
- ✅ `Box<dyn FnOnce() + Send + 'static>` = "any function" stored as a heap value — our `Job` type

---

## 🔭 What's Next?

**Step 5** — We write the actual code: `ThreadPool`, `Worker`, `Job`, and wire
the server to use `pool.execute(...)` instead of `thread::spawn(...)`.
