# Step 2 — Learning Rust Threads
> *"Like Python's `threading.Thread`, but with compile-time safety guarantees"*

---

## 🐍 Python Threads (What You Already Know)

In Python, spawning a thread looks like this:

```python
import threading

def do_work(data):
    print(f"Working on: {data}")

# Spawn a new thread
t = threading.Thread(target=do_work, args=("my data",))
t.start()
t.join() # Wait for thread to finish
```

Key things Python lets you do freely:
- Pass **any data** to a thread
- **Share data** between threads without restriction
- This freedom causes **race conditions** (two threads writing at the same time → corrupted data)

---

## 🦀 Rust Threads — `std::thread::spawn`

Rust's equivalent is `std::thread::spawn`. The API looks similar, but the **compiler enforces safety**:

```rust
use std::thread;

fn main() {
    // Spawn a new OS thread
    let handle = thread::spawn(|| {
        println!("Hello from a new thread!");
    });

    // Wait for the thread to finish (like Python's t.join())
    handle.join().unwrap();
}
```

### 🔑 Key Differences from Python

| Feature | Python `threading.Thread` | Rust `thread::spawn` |
| :--- | :--- | :--- |
| **Safety** | Runtime errors (race conditions possible) | Compile-time errors (races are impossible) |
| **Data sharing** | Implicit, anything goes | Must be explicit (`Arc`, `Mutex`) |
| **Join** | `t.join()` | `handle.join().unwrap()` |
| **Closure** | `target=fn, args=(...)` | A closure `\|\| { ... }` |
| **GIL** | Only one thread runs Python code at a time | True parallelism, all threads run simultaneously |

---

## 📦 The Closure — Rust's Way of Passing Work to a Thread

In Python you pass a `target` function and `args` separately.
In Rust, you pass a **closure** — a self-contained block of code that can *capture* variables from its environment.

```python
# Python: pass function and args separately
name = "Alice"
t = threading.Thread(target=greet, args=(name,))
```

```rust
// Rust: capture variables inside the closure
let name = String::from("Alice");

let handle = thread::spawn(move || {
    //                     ^^^^ 'move' transfers ownership of 'name' INTO the thread
    println!("Hello, {}!", name);
});
```

### ⚠️ Why `move`?

Without `move`, Rust asks: *"How long does `name` live? What if the main thread drops it before this thread reads it?"*

The `move` keyword **transfers ownership** of captured variables INTO the thread so it's guaranteed to be valid for the thread's entire lifetime.

```python
# Python doesn't care — shared references everywhere (but can crash at runtime)
name = "Alice"
t = threading.Thread(target=lambda: print(name))  # 'name' shared freely
```

```rust
// Rust forces you to be explicit about ownership
let name = String::from("Alice");
let handle = thread::spawn(move || {
    println!("{}", name); // 'name' is MOVED here — main thread can no longer use it
});
// println!("{}", name); // ❌ COMPILE ERROR: value moved into thread
```

---

## 🧵 The `JoinHandle` — Tracking Your Thread

`thread::spawn` returns a `JoinHandle<T>`. Think of it as a **receipt** for your thread.

```python
# Python
t = threading.Thread(target=work)
t.start()
t.join()  # block until done
```

```rust
// Rust
let handle: JoinHandle<()> = thread::spawn(|| {
    // do work
});
handle.join().unwrap(); // block until done, unwrap the Result
```

`join()` returns a `Result` because the thread might have **panicked**. `.unwrap()` propagates the panic to the main thread.

---

## ⚡ Spawning Multiple Threads

```python
# Python: spawn 4 threads
threads = []
for i in range(4):
    t = threading.Thread(target=worker, args=(i,))
    t.start()
    threads.append(t)

for t in threads:
    t.join()
```

```rust
// Rust: spawn 4 threads
use std::thread;

fn main() {
    let mut handles = vec![];

    for i in 0..4 {
        let handle = thread::spawn(move || {
            println!("Worker {} is running", i); // 'i' is copied (it's a number)
        });
        handles.push(handle);
    }

    // Wait for ALL threads to finish
    for handle in handles {
        handle.join().unwrap();
    }
}
```

---

## ⚠️ The Problem with Spawning Unlimited Threads

Spawning a new thread **per request** is the naive first solution:

```python
# Python naive approach
import threading, socket

server = socket.socket()
while True:
    conn, addr = server.accept()
    t = threading.Thread(target=handle, args=(conn,))
    t.start()
    # ☠️ 10,000 users = 10,000 threads = system crash
```

```rust
// Rust naive approach (same problem)
for stream in listener.incoming() {
    let stream = stream.unwrap();
    thread::spawn(move || {
        handle_connection(stream);
    });
    // ☠️ 10,000 users = 10,000 threads = out of memory
}
```

### Why unlimited threads are dangerous:

| Problem | Explanation |
| :--- | :--- |
| **Memory** | Each thread uses ~2MB of stack space. 1000 threads = 2GB RAM |
| **Context switching** | OS wastes time swapping between too many threads |
| **DoS vulnerability** | An attacker can crash your server by opening thousands of connections |

---

## ✅ Summary of Step 2

- ✅ `thread::spawn(|| { })` — spawns an OS thread, similar to Python's `threading.Thread`
- ✅ `move` closure — transfers ownership of variables *into* the thread (no dangling references)
- ✅ `JoinHandle` — a receipt for your thread; call `.join()` to wait for it
- ✅ Spawning one thread per request works, but is **dangerous** at scale
- ✅ The solution → a **Thread Pool** (Step 4)

---

## 🔭 What's Next?

**Step 3** — We'll build the **naive threaded server** (one thread per request) to see it work, then observe the problem firsthand before we build the proper Thread Pool.
