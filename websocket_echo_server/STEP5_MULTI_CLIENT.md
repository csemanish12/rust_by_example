# Step 5 — Multi-Client WebSocket Server with Thread Pool

> *"Reusing the ThreadPool we built from scratch — now every WebSocket client gets its own worker."*

---

## 🐍 The Python Equivalent We're Replicating

```python
from concurrent.futures import ThreadPoolExecutor
import websockets, asyncio

def handle_client(websocket):
    try:
        for message in websocket:
            websocket.send(message)   # echo back
    except Exception:
        pass  # client disconnected

with ThreadPoolExecutor(max_workers=4) as pool:
    server = socket.socket()
    server.bind(("127.0.0.1", 9001))
    server.listen()
    while True:
        conn, addr = server.accept()
        pool.submit(handle_client, conn)  # hand off to a worker
```

---

## 🆚 What Changed vs Step 3

### Step 3 — Single-threaded (one client blocks everyone)

```rust
// ❌ The main loop handles the client directly
for stream in listener.incoming() {
    handle_client(stream, peer); // main thread is STUCK here until client disconnects
                                 // all other clients wait
}
```

### Step 5 — Thread Pool (4 clients handled simultaneously)

```rust
// ✅ The main loop just dispatches — never blocks
for stream in listener.incoming() {
    pool.execute(move || {
        handle_client(stream, peer); // runs in a WORKER THREAD
    });               // main loop immediately accepts the next client
}
```

That's the **only change in `main()`**. The `handle_client` function itself is identical.

---

## 🏗️ Full Architecture — How Everything Connects

```
  main.rs                                  lib.rs
  ───────                                  ──────────────────────────────────────────
                                          ┌──────────────────────────────────────────┐
  TcpListener::bind(9001)                 │            ThreadPool (4 workers)        │
         │                               │                                          │
         │  for stream in incoming()     │  sender ──────────────────────────────►  │
         │       │                       │                                    channel│
         │       └── pool.execute(move|| {              Arc<Mutex<Receiver>>        │
         │               handle_client(stream, peer)           │                    │
         │           })  ───────────────────────────►  Worker 0 thread 😴/🔨        │
         │                                             Worker 1 thread 😴/🔨        │
         │  (main loop immediately goes                Worker 2 thread 😴/🔨        │
         │   back to accept next client)               Worker 3 thread 😴/🔨        │
         │                                          │                               │
         │                                          └── each worker runs:           │
         │                                              loop {                      │
         │                                                recv() → job()            │
         │                                                ↑ handle_client()         │
         │                                              }                           │
         └──────────────────────────────────────────────────────────────────────────┘

  Client 1 ──────────────────────────────────────────────────► Worker 0 🔨
  Client 2 ──────────────────────────────────────────────────► Worker 1 🔨
  Client 3 ──────────────────────────────────────────────────► Worker 2 🔨
  Client 4 ──────────────────────────────────────────────────► Worker 3 🔨
  Client 5 ──── waits in queue ────────────────────────────── (no free workers)
  Client 5 ──────────────────────────────────────────────────► Worker 0 🔨 (reused!)
```

---

## 🔍 Code Walkthrough — What's New

### `src/lib.rs` — ThreadPool (reused from multithreaded_server)

We copied the exact same `ThreadPool` we built in the multithreaded server project.
This demonstrates a core principle:

> **Good Rust code is modular and reusable.** The `ThreadPool` doesn't care whether
> it runs HTTP handlers or WebSocket handlers — it just runs any `FnOnce() + Send + 'static`.

```rust
pub struct ThreadPool {
    workers: Vec<Worker>,           // 4 threads waiting for work
    sender: Option<mpsc::Sender<Job>>, // the sending end of the job channel
}
```

### `src/main.rs` — Two Key Changes

#### Change 1: Create the pool instead of spawning threads

```rust
// Step 3 (old)
// No pool — threads spawned unboundedly per connection

// Step 5 (new)
let pool = ThreadPool::new(4);
// ☝️ exactly 4 threads created here, upfront, never more
```

#### Change 2: `pool.execute()` instead of direct call

```rust
// Step 3 (old)
handle_client(stream, peer);  // main thread blocks until done

// Step 5 (new)
pool.execute(move || {
    handle_client(stream, peer);  // worker thread handles it
});
// main thread returns here immediately — ready for next client
```

### `handle_client` — Extracted into its own function

```rust
fn handle_client(stream: std::net::TcpStream, peer: std::net::SocketAddr) {
    // WebSocket handshake
    let mut websocket = match accept(stream) { ... };

    // Echo loop — runs for the FULL LIFETIME of this client's connection
    loop {
        let message = match websocket.read() { ... };
        match message {
            Message::Text(...)   => { websocket.send(message.clone()); }
            Message::Close(...)  => { break; }
            ...
        }
    }
    // Function returns → worker thread loops back → ready for next client
}
```

In Step 3, this code was inline in `main()`. Now it's a **separate function** — the worker
calls it, and when it returns the worker is free again.

```python
# Python equivalent — same idea
def handle_client(conn, addr):
    ws = perform_handshake(conn)
    while True:
        msg = ws.recv()    # blocks for this client only
        ws.send(msg)

# Worker thread calls handle_client and becomes free when it returns
pool.submit(handle_client, conn, addr)
```

---

## 🔄 The Worker Lifecycle

```
Worker created  →  loop {
                       .lock().recv()     ← 😴 SLEEPING — waiting for a client
                                          ← client arrives → pool.execute() sends job
                       job()              ← 🔨 WORKING — inside handle_client()
                                             (could take seconds, minutes — doesn't matter)
                       (job returns)      ← ✅ DONE — client disconnected or sent Close
                   }                      ← 😴 back to sleeping — ready for next client
```

```python
# Python ThreadPoolExecutor does the exact same lifecycle internally
class Worker(threading.Thread):
    def run(self):
        while True:
            job = self.queue.get()  # 😴 sleeping
            job()                   # 🔨 working
            # ✅ done — loop back
```

---

## 🔐 Why `move` in `pool.execute(move || { ... })`

```rust
pool.execute(move || {
    handle_client(stream, peer);
});
```

Two variables are captured: `stream` (TcpStream) and `peer` (SocketAddr).

| Variable | Type | Why `move` |
| :--- | :--- | :--- |
| `stream` | `TcpStream` | Move type — must transfer ownership to the worker thread |
| `peer` | `SocketAddr` | Copy type — gets copied automatically |

Without `move`:
- `stream` would be a **reference** into the main thread's stack
- The worker might run AFTER the main thread has moved on
- Rust's compiler: ❌ *"I can't guarantee `stream` is still valid in the worker!"*

With `move`:
- `stream` is **owned by the closure** — lives exactly as long as the worker needs it
- Rust's compiler: ✅ *"Safe — the worker owns everything it needs."*

```python
# Python — this is invisible, everything is a shared reference
# (which is why Python can have race conditions)
pool.submit(handle_client, stream, peer)  # stream is passed, not moved
```

---

## 🧪 How to Test Multiple Clients

### Start the server:
```bash
cargo run
```

### Open 5 simultaneous WebSocket connections (Terminal 2):
```bash
# Install wscat once
npm install -g wscat

# Open 5 terminals and run this in each:
wscat -c ws://127.0.0.1:9001
```

### What you'll see in the server logs:
```
🚀 WebSocket Echo Server listening on ws://127.0.0.1:9001
   Workers: 4 (fixed thread pool)

[127.0.0.1:54001] TCP connection → sending to thread pool...
[Worker 0] picked up a WebSocket client.
[127.0.0.1:54001] ✅ WebSocket handshake complete!

[127.0.0.1:54002] TCP connection → sending to thread pool...
[Worker 1] picked up a WebSocket client.

[127.0.0.1:54003] TCP connection → sending to thread pool...
[Worker 2] picked up a WebSocket client.

[127.0.0.1:54004] TCP connection → sending to thread pool...
[Worker 3] picked up a WebSocket client.

[127.0.0.1:54005] TCP connection → sending to thread pool...
← Client 5 WAITS — all 4 workers are busy
← As soon as any worker finishes, Client 5 is picked up immediately
```

---

## 🆚 Full Project Comparison

| Feature | simple_server | multithreaded_server | websocket_echo_server |
| :--- | :--- | :--- | :--- |
| Protocol | HTTP over TCP | HTTP over TCP | **WebSocket over TCP** |
| Concurrency | ❌ Single client | ✅ Thread pool | ✅ Thread pool |
| Connection | Closes per request | Closes per request | **Persistent** |
| Server push | ❌ | ❌ | ✅ |
| Thread limit | 1 | Fixed (4) | Fixed (4) |
| External crate | None | None | tungstenite |

---

## ✅ Summary of Step 5

- ✅ **`lib.rs`** — ThreadPool copied directly from multithreaded_server (zero changes needed)
- ✅ **`pool.execute(move || { ... })`** — sends the entire client session to a free worker
- ✅ **`handle_client()`** — extracted into its own function; runs inside a worker thread
- ✅ **`move`** — transfers ownership of `stream` into the closure (compiler-enforced safety)
- ✅ **Worker lifecycle** — sleep → pick up client → handle → finish → sleep again
- ✅ **Graceful shutdown** — `Drop` on `ThreadPool` waits for all clients to finish cleanly
- ✅ **4 clients max simultaneous** — 5th client waits in the channel queue, never dropped

---

## 🔭 What's Next?

**Step 6** — Testing the server properly:
- `wscat` for interactive testing
- Browser DevTools console
- Sending binary frames
- Observing the thread pool behaviour under load
