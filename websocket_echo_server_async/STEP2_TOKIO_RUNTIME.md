# Step 2 — External Crates & the Async Runtime

> *"`tokio` is to Rust what `asyncio` is to Python — but faster, safer, and explicit."*

---

## 📦 `Cargo.toml` — What We Added

```toml
[dependencies]
tokio             = { version = "1", features = ["full"] }
tokio-tungstenite = "0.26"
futures-util      = "0.3"
```

---

## 🔍 Each Crate Explained

### `tokio` — The Async Runtime

```python
# Python — asyncio is built into the language
import asyncio
asyncio.run(main())   # event loop is hidden, automatic
```

```rust
// Rust — you choose your runtime explicitly
// tokio is the most popular choice
#[tokio::main]        // ← this macro sets up the event loop for you
async fn main() { }
```

Tokio provides:
- The **event loop** that drives all async tasks
- Async versions of stdlib types: `tokio::net::TcpListener`, `tokio::fs::File`, etc.
- `tokio::spawn()` — like `asyncio.create_task()`
- Timers: `tokio::time::sleep()` — like `asyncio.sleep()`

#### Why `features = ["full"]`?

```toml
# tokio is modular — only compile what you use
tokio = { features = ["full"] }      # everything (good for learning)
tokio = { features = ["net", "rt"] } # production — smaller binary, faster compile
```

```python
# Python equivalent
pip install uvicorn[standard]   # [standard] enables optional extras
```

---

### `tokio-tungstenite` — Async WebSocket

```
tungstenite          (sync)  ← used in websocket_echo_server
tokio-tungstenite    (async) ← used here

Same WebSocket protocol, different execution model:
  tungstenite:        websocket.read()          → BLOCKS the thread
  tokio-tungstenite:  receiver.next().await     → YIELDS to tokio
```

```python
# Python equivalent
websocket-client   (sync)    ← blocks
websockets         (async)   ← yields with await
```

Key function: `accept_async(stream).await` — performs the WebSocket handshake asynchronously.

---

### `futures-util` — Async Iterator Helpers

This crate gives us two critical traits:

```rust
use futures_util::StreamExt;  // adds .next().await  to the receiver
use futures_util::SinkExt;    // adds .send().await   to the sender
```

```python
# Python equivalent — these are built into websockets library
async for msg in websocket:        # StreamExt::next()
    await websocket.send(reply)    # SinkExt::send()
```

Without `StreamExt`, you can't call `.next()` on the WebSocket receiver.
Without `SinkExt`, you can't call `.send()` on the WebSocket sender.

---

## 🏗️ The `#[tokio::main]` Macro — What It Actually Does

```rust
// What you write:
#[tokio::main]
async fn main() {
    // your code
}
```

```rust
// What the macro expands to:
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // your async main code runs here
        });
}
```

It creates a **multi-threaded tokio runtime** — by default one thread per CPU core.
Each thread runs the event loop, picking up async tasks as they become ready.

```python
# Python equivalent
if __name__ == "__main__":
    asyncio.run(main())   # single-threaded event loop
    # (Python asyncio is single-threaded due to the GIL)
    # tokio is TRULY multi-threaded — no GIL in Rust!
```

---

## 🆚 tokio vs Python asyncio

| Feature | Python `asyncio` | Rust `tokio` |
| :--- | :--- | :--- |
| **Threads** | 1 (GIL) | 1 per CPU core (truly parallel) |
| **Task creation** | `asyncio.create_task()` | `tokio::spawn()` |
| **Sleep** | `await asyncio.sleep(1)` | `tokio::time::sleep(Duration::from_secs(1)).await` |
| **TCP listener** | `asyncio.start_server()` | `tokio::net::TcpListener::bind().await` |
| **Explicit runtime?** | No — `asyncio.run()` hides it | Yes — `#[tokio::main]` |
| **Performance** | Good | 2-10x faster |

---

## ✅ Summary

- ✅ `tokio` = the async runtime (event loop + thread pool) — like `asyncio` but truly multi-threaded
- ✅ `#[tokio::main]` = sets up the tokio runtime, makes `async fn main()` work
- ✅ `tokio-tungstenite` = async WebSocket — `.next().await` instead of blocking `.read()`
- ✅ `futures-util` = `StreamExt` (`.next()`) + `SinkExt` (`.send()`) for the split WebSocket
- ✅ `features = ["full"]` = all tokio features enabled — simplest for learning
