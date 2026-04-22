# Step 3 — The Async Echo Server

> *"Same logic as the sync server — but every blocking call becomes `.await`."*

---

## 🐍 Python Equivalent

```python
async def handle_client(websocket):
    async for message in websocket:       # yields while waiting
        await websocket.send(message)     # yields while sending
```

Our Rust server does exactly this — same shape, explicit ownership.

---

## 🔑 Key Concepts

### 1. `accept_async(stream).await` — Async Handshake

```rust
let ws_stream = accept_async(stream).await.unwrap();
//                                   ^^^^^
//              "pause here, yield to tokio, resume when handshake done"
```

> Same as `tungstenite::accept()` from the sync server — but non-blocking.

---

### 2. `ws_stream.split()` — Why We Must Split

```python
# Python — one object, read and write freely
async for msg in websocket:
    await websocket.send(msg)   # same object — no problem in Python
```

```rust
// Rust — can't have two mutable borrows of the same value at once
let (mut sender, mut receiver) = ws_stream.split();
//   ──────────  ────────────
//   write half  read half — now INDEPENDENT, Rust allows this
```

Rust's rule: **one mutable reference OR many immutable — never both**.
`split()` gives each half its own type, satisfying the borrow checker.

---

### 3. `while let Some(result) = receiver.next().await`

```python
async for msg in websocket:   # ends on disconnect (StopAsyncIteration)
```

```rust
while let Some(result) = receiver.next().await {
//     ^^^^^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^^^^^^
//     Some → message arrived, keep looping    |
//     None → stream closed (disconnect) ←─────┘
```

`.next().await` yields to tokio while waiting — other tasks run in the meantime.

---

### 4. `if let Err(e) = sender.send(...).await` — Send Error Handling

```python
try:
    await websocket.send(msg)
except Exception:
    break   # client dropped
```

```rust
if let Err(e) = sender.send(Message::Text(text)).await {
    break;  // client dropped — exit cleanly, no crash
}
```

---

### 5. `tokio::spawn(handle_client(stream, peer))` — No `move` Needed

```rust
// Passing a direct function call — stream and peer move IN at the call site
tokio::spawn(handle_client(stream, peer));

// vs. passing a closure — 'move' needed to capture variables
tokio::spawn(async move { handle_client(stream, peer).await; });
```

Both work. The direct call is cleaner when you have a named function.

---

## 🗺️ Async Flow

```
main loop                             tokio runtime
─────────                             ─────────────────────────────────────
listener.accept().await ──────────── [yield — wake me on new client]
new client arrives ◄──────────────── [woken up]
tokio::spawn(handle_client(...))  ── [new task created — runs independently]
back to accept().await ─────────────[yield again]

Task: Client 1                        Task: Client 2
  receiver.next().await → [yield] 😴   receiver.next().await → [yield] 😴
  msg arrives → [wake]                  msg arrives → [wake]
  sender.send().await → [yield]         sender.send().await → [yield]
  receiver.next().await → [yield] 😴   ...
```

All clients truly concurrent — zero blocking between them.

---

## 🆚 Sync vs Async at a Glance

| Operation | Sync (`websocket_echo_server`) | Async (this project) |
| :--- | :--- | :--- |
| Spawn per client | `thread::spawn` (~8MB) | `tokio::spawn` (~KB) |
| Wait for message | `websocket.read()` — blocks thread | `receiver.next().await` — yields |
| Send message | `websocket.send()` — blocks thread | `sender.send().await` — yields |
| Max clients | 4 (pool size) | Thousands |

---

## ✅ Summary

- ✅ `accept_async().await` — async handshake, yields while waiting
- ✅ `ws_stream.split()` — two independent halves to satisfy borrow checker
- ✅ `receiver.next().await` — yields to tokio, woken when message arrives
- ✅ `sender.send().await` — yields while sending
- ✅ `while let Some` — exits naturally when client disconnects (`None`)
- ✅ `if let Err` — handle send failures without crashing
- ✅ Direct function call to `tokio::spawn` — no `move` closure needed


```python
import asyncio
import websockets

async def handle_client(websocket):
    print(f"Client connected: {websocket.remote_address}")
    try:
        async for message in websocket:        # yields while waiting
            await websocket.send(message)      # yields while sending
    except websockets.ConnectionClosed:
        pass

async def main():
    async with websockets.serve(handle_client, "127.0.0.1", 9001):
        await asyncio.Future()  # run forever

asyncio.run(main())
```

---

## 🦀 Your Rust Code — Full Annotated

```rust
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::StreamExt;   // .next().await
use futures_util::SinkExt;     // .send().await
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:9001").await.unwrap();
    println!("🚀 Async WebSocket Echo Server");
    println!("   Listening on ws://127.0.0.1:9001");

    loop {
        let (stream, peer) = listener.accept().await.unwrap();
        println!("[{peer}] Connected!");

        tokio::spawn(async move {
            handle_client(stream, peer).await;
        });
    }
}

async fn handle_client(stream: TcpStream, peer: SocketAddr) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => { println!("[{peer}] Handshake failed: {e}"); return; }
    };
    println!("[{peer}] ✅ WebSocket handshake complete!");

    let (mut sender, mut receiver) = ws_stream.split();

    while let Some(result) = receiver.next().await {
        let msg = match result {
            Ok(m)  => m,
            Err(e) => { println!("[{peer}] Error: {e}"); break; }
        };

        match msg {
            Message::Text(text) => {
                println!("[{peer}] 📨 Got text: {text}");
                if let Err(e) = sender.send(Message::Text(text)).await {
                    println!("[{peer}] Error sending: {e}"); break;
                }
            }
            Message::Binary(data) => {
                println!("[{peer}] 📦 Got binary: {} bytes", data.len());
                if let Err(e) = sender.send(Message::Binary(data)).await {
                    println!("[{peer}] Error sending: {e}"); break;
                }
            }
            Message::Ping(ping) => {
                println!("[{peer}] 🏓 Ping received");
                if let Err(e) = sender.send(Message::Pong(ping)).await {
                    println!("[{peer}] Error sending pong: {e}"); break;
                }
            }
            Message::Close(_) => {
                println!("[{peer}] 👋 Client closed connection");
                break;
            }
            _ => {}
        }
    }

    println!("[{peer}] 🔌 Disconnected. Task ending, memory freed.");
}
```

---

## 🔍 Every New Concept Explained

### 1. `accept_async(stream).await` — Async Handshake

```python
# Python — websockets does this invisibly
async with websockets.serve(handler, host, port):
    ...
```

```rust
// Rust — explicit, but still one line
let ws_stream = accept_async(stream).await.unwrap();
//                                   ^^^^^
//                    "pause here, yield to tokio, resume when handshake done"
```

---

### 2. `ws_stream.split()` — Why We Must Split

```python
# Python — one websocket object, read and write freely
async for msg in websocket:
    await websocket.send(msg)   # same object, no problem
```

```rust
// Rust — CAN'T have two mutable borrows of the same value simultaneously
let (mut sender, mut receiver) = ws_stream.split();
//   ^^^^^^^^^^^^^^^^^^^^^^^^^
//   Now two INDEPENDENT halves — Rust allows this
//   sender   → write side (implements SinkExt   → .send().await)
//   receiver → read side  (implements StreamExt → .next().await)
```

This is a fundamental Rust rule:
> You can have ONE mutable reference OR many immutable references — never both.

`split()` solves this by creating two separate types that each own one half.

---

### 3. `while let Some(result) = receiver.next().await`

```python
# Python
async for msg in websocket:   # ends when client disconnects (StopAsyncIteration)
    ...
```

```rust
// Rust — more explicit but identical behaviour
while let Some(result) = receiver.next().await {
//     ^^^^^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^^^^^^
//     |                  "yield to tokio, wake me when next message arrives"
//     Some = message arrived → keep looping
//     None = stream ended (client disconnected) → loop exits naturally
```

---

### 4. `if let Err(e) = sender.send(...).await`

```python
# Python — exception if client dropped
await websocket.send(msg)   # raises WebSocketException

try:
    await websocket.send(msg)
except Exception as e:
    break
```

```rust
// Rust — error is a value, must be handled explicitly
if let Err(e) = sender.send(Message::Text(text)).await {
//  ^^^^^^^^^
//  "if the Result is an Err variant, bind the error to 'e'"
//  Ok(_) — send worked → continue loop
//  Err(e) — client dropped → log and break
    println!("[{peer}] Error sending: {e}");
    break;
}
```

---

### 5. `tokio::spawn` — Async Task vs OS Thread

```python
# Python
asyncio.create_task(handle_client(websocket))   # async task — lightweight
threading.Thread(target=handle_client).start()  # OS thread — heavyweight
```

```rust
// Rust
tokio::spawn(async move {          // async task — like asyncio.create_task()
    handle_client(stream, peer).await;
});

thread::spawn(move || {            // OS thread — like threading.Thread()
    handle_client(stream, peer);
});
```

| | `tokio::spawn` | `thread::spawn` |
| :--- | :--- | :--- |
| **Memory per task** | ~KB | ~8MB |
| **10,000 of them** | ✅ Fine | ❌ Crash |
| **Blocks on `.await`?** | ✅ Yields | N/A |
| **Python equivalent** | `asyncio.create_task()` | `threading.Thread()` |

---

## 🗺️ Full Async Flow Diagram

```
  main()                                    tokio runtime
  ──────                                    ──────────────────────────────────
  TcpListener::bind().await
         │
  loop {
      listener.accept().await  ────────────► [yield] → tokio runs other tasks
                               ◄──────────── client arrived → resume
      tokio::spawn(async move {
          handle_client().await ───────────► [new async task created]
      });                                    task runs on any available thread
  }                                         independently of main loop

                                            Task for Client 1:
                                              accept_async().await → [yield]
                                              receiver.next().await → [yield] 😴
                                              (tokio wakes it when msg arrives)
                                              sender.send().await → [yield]
                                              receiver.next().await → [yield] 😴
                                              ...forever until disconnect

                                            Task for Client 2: same, independent
                                            Task for Client 3: same, independent
                                            ...
                                            Task for Client 10,000: same ✅
```

---

## 🆚 Sync vs Async — Side by Side

```
websocket_echo_server (sync)        websocket_echo_server_async (async)
────────────────────────────        ───────────────────────────────────

thread::spawn(move || {             tokio::spawn(async move {
    handle_client(stream);              handle_client(stream).await;
});                                 });

websocket.read()                    receiver.next().await
  → BLOCKS the thread                 → YIELDS to tokio runtime
  → thread frozen, unusable          → thread free for other tasks
  → 4 workers = 4 clients            → 1 thread = thousands of clients

websocket.send(msg)                 sender.send(msg).await
  → BLOCKS until sent                 → YIELDS while sending
```

---

## ✅ Summary of Step 3

- ✅ `accept_async(stream).await` — async WebSocket handshake
- ✅ `ws_stream.split()` — splits into `sender` + `receiver` (Rust's borrow rules)
- ✅ `receiver.next().await` — yield while waiting (like Python's `async for msg in ws`)
- ✅ `sender.send(msg).await` — yield while sending
- ✅ `while let Some(result)` — loops until client disconnects (returns `None`)
- ✅ `if let Err(e)` — handle send errors without crashing the server
- ✅ `tokio::spawn` — spawns a lightweight async task per client (not an OS thread)
- ✅ Every client runs in its own task — truly concurrent, minimal memory cost

---

## 🔭 What's Next?

**Step 4** — Production features:
- Heartbeat / Ping-Pong to detect dead connections
- Connection timeouts (drop idle clients after X minutes)
- Graceful shutdown with `tokio::signal`
- Logging with `tracing`
