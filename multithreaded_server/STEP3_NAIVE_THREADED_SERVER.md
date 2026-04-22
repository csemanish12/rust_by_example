# Step 3 — The Naive Threaded Server (One Thread Per Request)

> *"It works! But it has a fatal flaw."*

---

## 🐍 The Python Equivalent We're Replicating

```python
import socket
import threading
import time

def handle_connection(conn):
    data = conn.recv(1024).decode()
    print(f"[{threading.current_thread().name}] Got: {data.splitlines()[0]}")
    time.sleep(2)  # Simulate slow work
    conn.sendall(b"HTTP/1.1 200 OK\r\n\r\n<h1>Hello!</h1>")
    conn.close()

server = socket.socket()
server.bind(("127.0.0.1", 7878))
server.listen()

while True:
    conn, addr = server.accept()
    t = threading.Thread(target=handle_connection, args=(conn,))
    t.start()
    # ☝️ A brand new thread for EVERY connection
```

Our Rust server does **exactly** this — one thread per request.

---

## 🦀 The Rust Code — What Changed vs Simple Server

### Before (Simple Server — single threaded):
```rust
for stream in listener.incoming() {
    let stream = stream.unwrap();
    handle_connection(stream); // 😴 BLOCKS — nothing else runs until done
}
```

### After (Naive Threaded Server):
```rust
for stream in listener.incoming() {
    let stream = stream.unwrap();

    thread::spawn(move || {      // 🚀 Spawn a NEW thread
        handle_connection(stream); // Runs independently, main loop keeps going
    });
}
```

That's the **entire change** — wrap `handle_connection` in `thread::spawn(move || { })`.

---

## 🔍 Line-by-Line Explanation

### `thread::spawn(...)`
```rust
thread::spawn(move || {
    handle_connection(stream);
});
```
- Creates a brand new **OS thread**
- Returns a `JoinHandle` — but we deliberately **ignore** it here
- The main loop immediately moves on to accept the next connection

> ⚠️ We ignore the `JoinHandle` on purpose — we don't want the main thread
> to wait. Each thread lives and dies on its own.

---

### `move ||`
```rust
thread::spawn(move || {
```
- `stream` is a `TcpStream` — a **Move type** (not Copy)
- Without `move`, Rust asks: *"How do you know `stream` will still be valid inside the thread?"*
- With `move`, ownership of `stream` is **transferred** into the thread — guaranteed safe

```python
# Python doesn't need this — shared references are implicit
# But that's also why Python can have race conditions!
```

---

### `thread::current().id()`
```rust
println!("[Thread {:?}] Got request: {}", thread::current().id(), request_line);
```
- Gets the ID of the **currently executing thread**
- When you make multiple requests, you'll see **different thread IDs** in the logs — proof that requests are being handled in parallel

---

### `thread::sleep(Duration::from_secs(2))`
```rust
thread::sleep(Duration::from_secs(2));
```
- Simulates a **slow operation** (database query, file read, etc.)
- In the old single-threaded server, this would freeze everything for 2 seconds
- Now, try hitting the server 3 times quickly — all 3 respond at roughly the same time!

---

## 🧪 How to Test It

### Run the server:
```bash
cargo run
```

### Test with curl in 3 separate terminals simultaneously:
```bash
# Terminal 1
curl http://127.0.0.1:7878

# Terminal 2 (same time)
curl http://127.0.0.1:7878

# Terminal 3 (same time)
curl http://127.0.0.1:7878
```

### What you'll see in server logs:
```
🚀 Naive Threaded Server running on http://127.0.0.1:7878
[Thread ThreadId(2)] Got request: GET / HTTP/1.1
[Thread ThreadId(3)] Got request: GET / HTTP/1.1   ← different thread!
[Thread ThreadId(4)] Got request: GET / HTTP/1.1   ← another thread!
[Thread ThreadId(2)] Done.
[Thread ThreadId(3)] Done.
[Thread ThreadId(4)] Done.
```

All 3 requests are handled **simultaneously** — each with a unique thread ID. 🎉

---

## ❌ The Problem: What Happens Under Load?

```python
# Simulate 10,000 users hitting the server at once
import threading
threads = [threading.Thread(target=make_request) for _ in range(10_000)]
for t in threads: t.start()
# ☠️ System grinds to a halt — 10,000 threads × 2MB stack = 20GB RAM
```

```rust
// Rust does the same — 10,000 connections = 10,000 threads spawned
for stream in listener.incoming() {
    thread::spawn(move || { handle_connection(stream.unwrap()); });
    // ☠️ No limit — attacker can crash your server with a simple script
}
```

### The Numbers:

| Threads Spawned | RAM Used (approx) | What Happens |
| :--- | :--- | :--- |
| 10 | ~20 MB | Fine ✅ |
| 100 | ~200 MB | Fine ✅ |
| 1,000 | ~2 GB | Slow ⚠️ |
| 10,000 | ~20 GB | Crash ❌ |

---

## ✅ Summary of Step 3

| | Simple Server | Naive Threaded Server |
| :--- | :--- | :--- |
| **Concurrency** | ❌ One at a time | ✅ Many at a time |
| **Blocking** | ❌ Blocks on each request | ✅ Non-blocking main loop |
| **Thread limit** | N/A (1 thread) | ❌ Unlimited — dangerous |
| **Memory safe?** | ✅ | ✅ (Rust still guarantees this) |
| **Production ready?** | ❌ | ❌ |

---

## 🔭 What's Next?

**Step 4** — We learn about **Thread Pools**: a fixed number of worker threads that
*reuse* themselves instead of spawning new ones. Like Python's `ThreadPoolExecutor`.

```python
# Step 4 preview — what we're building towards
from concurrent.futures import ThreadPoolExecutor

with ThreadPoolExecutor(max_workers=4) as pool:
    pool.submit(handle_connection, conn)  # max 4 threads, ever
```
