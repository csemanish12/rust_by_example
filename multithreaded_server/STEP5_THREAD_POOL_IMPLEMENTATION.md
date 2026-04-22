# Step 5 — Building the Thread Pool from Scratch

> *"We build what Python's `ThreadPoolExecutor` does internally — in Rust."*

---

## 🏗️ The Full ASCII Architecture Diagram

```
  main.rs                          lib.rs
  ───────                          ──────────────────────────────────────────────────────────
                                  ┌─────────────────────────────────────────────────────────┐
                                  │                   ThreadPool                            │
                                  │                                                         │
  let pool = ThreadPool::new(4)──►│  workers: Vec<Worker>  ←── 4 Worker structs             │
                                  │  sender:  Sender<Job>  ─────────────────────────────┐   │
                                  │                                                     │   │
  pool.execute(move || {          │                                                     │   │
      handle_connection(stream)   │  fn execute(f) {                                   │   │
  })  ─────────────────────────► │      sender.send(Box::new(f))  ────────────────────►│   │
                                  │  }                                                  │   │
                                  └─────────────────────────────────────────────────────│───┘
                                                                                        │
                                              mpsc::channel  (the pipe)                │
                                  ┌─────────────────────────────────────────────────────▼───┐
                                  │                                                         │
                                  │   Sender ──────────────────────────► Receiver           │
                                  │   (ThreadPool owns this)              (Workers share)   │
                                  │                                                         │
                                  └────────────────────────────┬────────────────────────────┘
                                                               │
                                             Arc<Mutex<Receiver<Job>>>
                                             (one shared, locked receiver)
                                                               │
                                        ┌──────────────────────┼───────────────────────┐
                                        │                       │                       │
                                        ▼                       ▼                       ▼
                              ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
                              │    Worker 0     │   │    Worker 1     │   │    Worker 2     │  ...
                              │  ┌───────────┐  │   │  ┌───────────┐  │   │  ┌───────────┐  │
                              │  │  thread   │  │   │  │  thread   │  │   │  │  thread   │  │
                              │  │           │  │   │  │           │  │   │  │           │  │
                              │  │  loop {   │  │   │  │  loop {   │  │   │  │  loop {   │  │
                              │  │  .lock()  │  │   │  │  .lock()  │  │   │  │  .lock()  │  │
                              │  │  .recv()  │  │   │  │  .recv()  │  │   │  │  .recv()  │  │
                              │  │  job()    │  │   │  │  job()    │  │   │  │  job()    │  │
                              │  │  }        │  │   │  │  }        │  │   │  │  }        │  │
                              │  └───────────┘  │   │  └───────────┘  │   │  └───────────┘  │
                              └─────────────────┘   └─────────────────┘   └─────────────────┘
                                      ▲                      ▲
                                      │                      │
                              Worker 0 is busy      Worker 1 is free
                              (handling request)    (waiting on recv())
```

---

## 🧩 The Three Structs We Built

### 1. `Job` — Type Alias

```rust
type Job = Box<dyn FnOnce() + Send + 'static>;
```

```
Job
 │
 ├── Box<...>           → heap pointer of fixed size (closures vary in size)
 ├── dyn FnOnce()       → any callable, runs ONCE (our handler consumes the TcpStream)
 ├── + Send             → safe to transfer to another thread
 └── + 'static          → no short-lived borrows (thread may outlive the caller)
```

**Python equivalent:**
```python
Job = Callable  # any function object — Python doesn't need the box/send/static constraints
```

---

### 2. `Worker` — The Thread That Loops Forever

```rust
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}
```

```
Worker
 │
 ├── id        → just a number for logging ("Worker 0", "Worker 1", ...)
 └── thread    → Option<JoinHandle>
                  │
                  ├── Option  → so we can take() the handle during shutdown (Drop)
                  └── JoinHandle → the actual OS thread; .join() waits for it to finish
```

**Inside `Worker::new` — the loop:**

```rust
loop {
    let message = receiver   // Arc<Mutex<Receiver<Job>>>
        .lock()              // Step 1: acquire Mutex → only THIS worker reads now
        .unwrap()            // Step 2: unwrap (panic if another thread panicked)
        .recv();             // Step 3: BLOCK until a Job arrives, then release lock

    match message {
        Ok(job)  => job(),   // got a job → run it
        Err(_)   => break,   // sender dropped → pool shutting down → exit loop
    }
}
```

> ⚠️ **Key detail:** `.lock()` is released after `.recv()` returns (end of the chained
> expression). This means the Mutex is NOT held while the job runs — other workers can
> receive their own jobs concurrently.

**Python equivalent:**
```python
class Worker(threading.Thread):
    def run(self):
        while True:
            try:
                job = self.queue.get()  # blocks — like recv()
                job()
            except ShutdownError:
                break
```

---

### 3. `ThreadPool` — The Public API

```rust
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}
```

```
ThreadPool
 │
 ├── workers   → Vec of all Worker structs (holds the JoinHandles)
 └── sender    → Option<Sender<Job>>
                  │
                  ├── Option  → so we can drop() it during shutdown to close the channel
                  └── Sender  → the sending end of the mpsc channel
```

**`ThreadPool::new(4)` step by step:**

```
1. mpsc::channel()  →  creates (sender, receiver)
                                         │
2. Arc::new(Mutex::new(receiver))        │  wrap receiver so it can be shared
                         │               │
3. for i in 0..4:        │               │
       Arc::clone(...)  ─┼───────────────┤  cheap clone (just increments ref count)
       Worker::new(i, clone)             │  each worker gets its own Arc pointer
                                         │  all pointing to the SAME receiver
4. ThreadPool { workers, sender: Some(sender) }
```

**`pool.execute(f)` step by step:**

```
1. Box::new(f)              → heap-allocate the closure (fixed size pointer)
2. sender.send(boxed_job)   → push job into the channel
3. One waiting worker:
       .lock()  → acquires mutex
       .recv()  → unblocks, gets the job
       lock released
       job()    → runs handle_connection(stream)
```

---

## 🔐 Why `Arc<Mutex<Receiver>>` — The Deep Explanation

```
The Problem:
  We have 4 workers. All 4 need to read from the SAME receiver.
  But Rust only allows ONE owner of a value.

  Worker 0 owns receiver? → Workers 1,2,3 can't use it.  ❌
  Each worker gets a clone? → They'd each have a separate queue, jobs lost. ❌

The Solution: Arc<Mutex<Receiver>>

  Arc  solves OWNERSHIP:
  ┌─────────────────────────────────────────────────────────┐
  │                 Arc (ref count = 4)                     │
  │                                                         │
  │  Worker 0 ──► Arc ptr ─┐                                │
  │  Worker 1 ──► Arc ptr ─┼──► Mutex<Receiver>  (1 copy)  │
  │  Worker 2 ──► Arc ptr ─┤                                │
  │  Worker 3 ──► Arc ptr ─┘                                │
  └─────────────────────────────────────────────────────────┘
  All 4 workers own the Arc. The Receiver is dropped when the last Arc is dropped.

  Mutex solves CONCURRENT ACCESS:
  ┌──────────────────────────────────────────────────────────┐
  │  Worker 0: .lock() → ✅ got lock → .recv() → got job    │
  │  Worker 1: .lock() → ⏳ blocked  → waiting...           │
  │  Worker 2: .lock() → ⏳ blocked  → waiting...           │
  │  Worker 3: .lock() → ⏳ blocked  → waiting...           │
  │                                                          │
  │  Worker 0 releases lock (recv() returned)               │
  │  Worker 1: .lock() → ✅ got lock → .recv() → waiting..  │
  └──────────────────────────────────────────────────────────┘
  Only ONE worker calls recv() at a time. No race conditions. Ever.
```

**Python comparison:**
```python
# Python's queue.Queue does all of this internally — you never see it
job_queue = queue.Queue()  # thread-safe by default (has an internal Mutex)
# Rust makes you write it explicitly — but that's what makes it provably safe
```

---

## 🔄 Graceful Shutdown — The `Drop` Trait

```rust
impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take()); // 1. close the channel
        for worker in &mut self.workers {
            worker.thread.take().unwrap().join().unwrap(); // 2. wait for each worker
        }
    }
}
```

```
Shutdown sequence:
  1. sender.take()    → removes sender from Option, drops it
                         channel is now CLOSED
                         all workers blocked on recv() get Err(_)
                         all workers break their loop and exit

  2. thread.join()    → main thread WAITS for each worker to finish
                         any in-progress job completes before shutdown
                         no requests are dropped mid-flight
```

**Python equivalent:**
```python
# Python's context manager does this automatically
with ThreadPoolExecutor(max_workers=4) as pool:
    ...
# __exit__ calls pool.shutdown(wait=True)
#   → stops accepting new jobs
#   → waits for all running jobs to complete
```

---

## 🆚 Before vs After

```
STEP 3 (Naive)                      STEP 5 (Thread Pool)
──────────────                      ────────────────────

Request  1 → thread::spawn()  →  Thread  1   |  Request  1 → pool.execute() → Worker 0
Request  2 → thread::spawn()  →  Thread  2   |  Request  2 → pool.execute() → Worker 1
Request  3 → thread::spawn()  →  Thread  3   |  Request  3 → pool.execute() → Worker 2
Request  4 → thread::spawn()  →  Thread  4   |  Request  4 → pool.execute() → Worker 3
...                                           |  Request  5 → pool.execute() → (waits)
Request 10000 → Thread 10000 ← 💀 CRASH      |  Request  5 → pool.execute() → Worker 0 (reused)
```

| | Naive (Step 3) | Thread Pool (Step 5) |
| :--- | :--- | :--- |
| **Max threads** | Unlimited ❌ | Fixed (4) ✅ |
| **Memory** | Grows forever ❌ | Capped ✅ |
| **DoS resistant** | No ❌ | Yes ✅ |
| **Graceful shutdown** | No ❌ | Yes ✅ |
| **Production ready** | No ❌ | Yes ✅ |

---

## 📁 Files Changed

| File | What changed |
| :--- | :--- |
| `src/lib.rs` | **New** — `ThreadPool`, `Worker`, `Job` implementation |
| `src/main.rs` | `thread::spawn()` replaced with `pool.execute()` |

---

## 🧪 How to Test

```bash
cargo run
```

Open 3 terminals and run simultaneously:
```bash
curl http://127.0.0.1:7878   # Terminal 1
curl http://127.0.0.1:7878   # Terminal 2
curl http://127.0.0.1:7878   # Terminal 3
```

You'll see in the server logs:
```
🚀 Thread Pool Server running on http://127.0.0.1:7878
   Workers: 4 (fixed — no unlimited spawning)
[Worker 0] Received a job. Executing...
[Worker 1] Received a job. Executing...
[Worker 2] Received a job. Executing...
```

Notice: thread IDs **recycle** — Worker 0 handles request 1 AND request 5.
With the naive server, every request got a brand new thread ID.

---

## ✅ Summary of Step 5

- ✅ `type Job` = `Box<dyn FnOnce() + Send + 'static>` — any sendable one-shot closure
- ✅ `Worker` = struct wrapping an OS thread that loops on `recv()`
- ✅ `ThreadPool` = holds `Vec<Worker>` + `Sender<Job>`
- ✅ `Arc<Mutex<Receiver>>` = how all workers safely share one receiver
- ✅ `Drop` trait = graceful shutdown — no requests lost, no threads leaked
- ✅ `pool.execute(f)` = `sender.send(Box::new(f))` — the whole public API
