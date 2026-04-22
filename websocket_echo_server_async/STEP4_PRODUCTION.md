# Step 4 — Production Features

> *"A server that doesn't crash, handles dead clients, and shuts down gracefully."*

---

## 🔴 Problems With Our Current Server

```
Problem 1: Dead connections
  Client closes laptop (no Close frame sent)
  → our server waits on receiver.next().await FOREVER
  → task leaks, memory grows, never cleaned up

Problem 2: No shutdown story
  CTRL+C kills the process instantly
  → in-flight messages dropped
  → no cleanup

Problem 3: Unwrap() everywhere
  listener.accept().await.unwrap()
  → one bad network event crashes the WHOLE server
```

---

## ✅ What We're Adding

```
1. Heartbeat (Ping/Pong)     → detect dead clients, drop them cleanly
2. Graceful shutdown          → finish in-flight work, then exit cleanly
3. Connection timeout         → drop clients that never send anything
```

---

## 🔧 New Crates Needed

Add these to your `Cargo.toml` under `[dependencies]`:

```toml
tokio            = { version = "1", features = ["full"] }
tokio-tungstenite = "0.26"
futures-util     = "0.3"
tracing          = "0.1"
tracing-subscriber = "0.3"
```

| Crate | Python Equivalent | Purpose |
| :--- | :--- | :--- |
| `tracing` | `logging` | Structured logging (replaces `println!`) |
| `tracing-subscriber` | `logging.basicConfig()` | Sets up where logs go |

---

## 🧠 New Rust Concepts You'll Hit

### 1. `tokio::select!` — Race Multiple Async Operations

```python
# Python — wait for EITHER a message OR a timeout
try:
    msg = await asyncio.wait_for(websocket.recv(), timeout=30.0)
except asyncio.TimeoutError:
    print("No message in 30s — dropping client")
```

```rust
// Rust — tokio::select! races futures against each other
// Whichever completes FIRST wins — the others are cancelled
tokio::select! {
    result = receiver.next() => {
        // a message arrived
    }
    _ = tokio::time::sleep(Duration::from_secs(30)) => {
        // 30 seconds passed with no message
        println!("Client timed out");
        break;
    }
}
```

> `select!` = "start ALL of these futures, run whichever finishes first, cancel the rest"
> Python's `asyncio.wait()` with `return_when=FIRST_COMPLETED` is the equivalent.

---

### 2. `tokio::signal::ctrl_c()` — Catch CTRL+C Gracefully

```python
# Python
import signal, asyncio

async def shutdown(signal):
    print(f"Got {signal}, shutting down...")
    # cleanup code

loop = asyncio.get_event_loop()
loop.add_signal_handler(signal.SIGINT, lambda: asyncio.create_task(shutdown("SIGINT")))
```

```rust
// Rust — tokio gives us an async future that resolves when CTRL+C is pressed
tokio::signal::ctrl_c().await.unwrap();
println!("Shutting down...");
```

---

### 3. `tracing` — Structured Logging

```python
# Python
import logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)
logger.info("Client connected", extra={"peer": addr})
```

```rust
// Rust
use tracing::{info, warn, error};

info!(peer = %peer, "Client connected");      // INFO level
warn!(peer = %peer, "Client timed out");      // WARN level
error!(peer = %peer, error = %e, "Failed");   // ERROR level
```

Structured logging means each field is **queryable** — useful with tools like Grafana, Datadog, etc.

---

### 4. `tokio::time::interval` — Repeating Timer

```python
# Python — send a ping every 20 seconds
async def heartbeat(ws):
    while True:
        await asyncio.sleep(20)
        await ws.ping()
```

```rust
// Rust — interval fires repeatedly
let mut ping_interval = tokio::time::interval(Duration::from_secs(20));

loop {
    tokio::select! {
        _ = ping_interval.tick() => {
            // 20 seconds passed → send a ping
            sender.send(Message::Ping(vec![])).await?;
        }
        result = receiver.next() => {
            // message received → handle it
        }
    }
}
```

---

## 🏗️ Architecture of the New `handle_client`

```
handle_client(stream, peer)
      │
      ├── accept_async()        →  WebSocket handshake
      ├── ws_stream.split()     →  sender + receiver
      ├── ping_interval         →  fires every 20 seconds
      │
      └── loop {
              tokio::select! {
                  ┌─────────────────────────────────────────────┐
                  │  Branch 1: receiver.next()                  │
                  │    Some(Ok(msg))  → echo it back            │
                  │    Some(Err(e))   → log error, break        │
                  │    None           → client gone, break      │
                  ├─────────────────────────────────────────────┤
                  │  Branch 2: ping_interval.tick()             │
                  │    → send Ping to client                    │
                  │    → if client dead → send fails → break    │
                  └─────────────────────────────────────────────┘
              }
          }

info!("Disconnected")   ← runs ONCE after loop exits
```

---

## 🔍 What Good Logs Look Like

```
INFO peer=127.0.0.1:54321 New TCP connection
INFO peer=127.0.0.1:54321 ✅ WebSocket connected
INFO peer=127.0.0.1:54321 msg=hello 📨 Text
INFO peer=127.0.0.1:54321 Sending heartbeat Ping
INFO peer=127.0.0.1:54321 👋 Close frame received
INFO peer=127.0.0.1:54321 🔌 Disconnected. Task cleaned up.

^C
INFO CTRL+C received — shutting down gracefully...
INFO Server stopped.
```

---

## ✅ Summary of Step 4

- ✅ `tokio::select!` — race multiple async futures, first one wins
- ✅ `tokio::time::interval` — repeating timer for heartbeat pings
- ✅ `ping_interval.tick().await` — consume the first immediate tick before the loop
- ✅ `tokio::signal::ctrl_c()` — async future that resolves on CTRL+C
- ✅ `tokio::pin!` — required when polling a future multiple times in `select!`
- ✅ `tracing` — structured logging with fields (`peer = %peer`)
- ✅ `handle_message()` — extracted into its own function, returns `bool`
- ✅ Dead clients detected via failed Ping → `break` → task drops → memory freed
- ✅ Graceful shutdown — server stops accepting, existing tasks finish naturally
