# Step 3 — Building the WebSocket Echo Server

> *"Accept a connection, upgrade to WebSocket, echo every message back."*

---

## 🐍 The Python Version We're Replicating

```python
import asyncio
import websockets

async def echo(websocket):
    print(f"Client connected: {websocket.remote_address}")
    try:
        async for message in websocket:       # read messages in a loop
            print(f"Received: {message}")
            await websocket.send(message)     # echo it back
            print(f"Echoed back.")
    except websockets.exceptions.ConnectionClosed:
        print("Client disconnected.")

async def main():
    async with websockets.serve(echo, "127.0.0.1", 9001):
        print("WebSocket server on ws://127.0.0.1:9001")
        await asyncio.Future()  # run forever

asyncio.run(main())
```

Our Rust server does **exactly** this — without async.

---

## 🦀 The Full Rust Code — Annotated

```rust
use std::net::TcpListener;
use tungstenite::accept;
use tungstenite::Message;
```

### The `use` keyword — importing names into scope

```
use std::net::TcpListener;   →  from stdlib,     import TcpListener
use tungstenite::accept;     →  from tungstenite, import accept()
use tungstenite::Message;    →  from tungstenite, import Message enum
```

```python
# Python equivalent
from socket import socket           # TcpListener
import websockets                    # accept
from websockets import Message       # (Python doesn't have this explicitly)
```

---

## 🔍 Step-by-Step Code Walkthrough

### Step 1 — Bind the TCP Listener (you already know this!)

```rust
let listener = TcpListener::bind("127.0.0.1:9001").unwrap();
```

```python
server = socket.socket()
server.bind(("127.0.0.1", 9001))
server.listen()
```

Same as your simple_server — WebSocket still starts with a raw TCP socket.

---

### Step 2 — Accept a TCP Connection

```rust
for stream in listener.incoming() {
    let stream = stream.unwrap();
    let peer = stream.peer_addr().unwrap(); // "192.168.1.5:54321"
```

```python
conn, addr = server.accept()
```

`peer_addr()` gives us the client's IP and port — useful for logging.

---

### Step 3 — The WebSocket Handshake with `accept()`

```rust
let mut websocket = match accept(stream) {
    Ok(ws) => ws,
    Err(e) => {
        println!("[{peer}] Handshake failed: {e}");
        continue;
    }
};
```

This is where TCP becomes WebSocket. `tungstenite::accept()`:
1. Reads the HTTP `Upgrade: websocket` request from the client
2. Validates the `Sec-WebSocket-Key` header
3. Sends back `HTTP/1.1 101 Switching Protocols`
4. Returns a `WebSocket<TcpStream>` — your new WebSocket handle

```python
# Python — websockets library does this automatically
async with websockets.serve(handler, host, port):
    ...
# The handshake is invisible to you — tungstenite.accept() is the equivalent
```

#### Why `match` instead of `.unwrap()`?

```rust
// Option A — crash on error (bad for a server)
let websocket = accept(stream).unwrap(); // ❌ panics if client sends bad data

// Option B — match (handle error gracefully)
let websocket = match accept(stream) {
    Ok(ws)  => ws,           // ✅ handshake succeeded
    Err(e)  => { continue; } // ✅ bad client? skip them, keep serving
};
```

```python
# Python equivalent
try:
    ws = await perform_handshake(conn)
except Exception as e:
    print(f"Handshake failed: {e}")
    continue
```

A server must **never crash** on bad input. `match` is Rust's structured
`try/except` — the compiler forces you to handle the error case.

---

### Step 4 — The Echo Loop

```rust
loop {
    let message = match websocket.read() {
        Ok(msg) => msg,
        Err(e)  => { break; } // client disconnected
    };

    // ... handle the message
}
```

```python
# Python equivalent
async for message in websocket:   # StopAsyncIteration on disconnect
    ...
# or
while True:
    try:
        message = await websocket.recv()
    except websockets.exceptions.ConnectionClosed:
        break
```

`websocket.read()` **blocks** until:
- A message arrives → returns `Ok(Message)`
- The connection drops → returns `Err(...)` → we `break` the loop

---

### Step 5 — Pattern Matching on `Message` (the Enum)

This is the most important new Rust concept in this step.

```rust
match message {
    Message::Text(ref text)   => { websocket.send(message.clone()).unwrap(); }
    Message::Binary(ref bytes) => { websocket.send(message.clone()).unwrap(); }
    Message::Ping(_)           => { /* tungstenite auto-sends Pong */ }
    Message::Close(_)          => { break; }
    _                          => { /* ignore */ }
}
```

#### What is an `enum`?

In Python, you'd check the type of a value at runtime:
```python
if isinstance(message, str):     # Text
    ...
elif isinstance(message, bytes): # Binary
    ...
```

In Rust, `Message` is an **enum** — a type that can be one of several variants,
each carrying its own data:

```
Message  (the enum)
 │
 ├── Message::Text(String)      → variant carrying a String
 ├── Message::Binary(Vec<u8>)   → variant carrying bytes
 ├── Message::Ping(Vec<u8>)     → variant carrying ping payload
 ├── Message::Pong(Vec<u8>)     → variant carrying pong payload
 └── Message::Close(Option<CloseFrame>) → variant with close info
```

`match` **destructs** the enum — it extracts the inner data AND handles every case.
The compiler forces you to cover all variants (or use `_` as a catch-all).

```rust
match message {
    Message::Text(ref text) => {
    //            ──────── ← 'text' is now a &String — extracted from the variant
        println!("Got text: {text}");
    }
    _ => {} // catch-all — handles all other variants
}
```

#### `ref` — why not just `text`?

```rust
Message::Text(ref text)  // ← borrow the inner String (don't move it)
//            ───
// Without 'ref': text would be MOVED out of message — then we can't clone message
// With 'ref':    text is borrowed — message still owns the String — we can clone it
```

```python
# Python doesn't have this — all variables are references by default
text = message  # just another reference, original is fine
```

#### `message.clone()` — why do we clone?

```rust
websocket.send(message.clone()).unwrap();
//                     ─────
// send() takes ownership of the Message (it's moved into send)
// We borrow with 'ref text' above to inspect it
// Then clone the whole message to send it
// Without clone: we'd have to reconstruct the message from 'text'
```

---

## 🗺️ Full Flow Diagram

```
  Client                              Server (main.rs)
  ──────                              ────────────────
  TCP connect()           ──────────► listener.incoming()
                                       │
                                       └── stream = stream.unwrap()
                                           peer   = stream.peer_addr()

  "GET / HTTP/1.1         ──────────►
   Upgrade: websocket"                 tungstenite::accept(stream)
  "101 Switching          ◄──────────     └── reads HTTP upgrade
   Protocols"                              └── sends 101 response
                                            └── returns WebSocket handle

  "Hello!"                ──────────► loop {
                                          websocket.read()
                          ◄──────────     websocket.send(message.clone())
  "Hello!" (echoed back)              }

  [Close frame]           ──────────► match Message::Close(_) → break
                          ◄──────────     websocket.send(Message::Close(None))
  [Close frame]
```

---

## 🆚 Python vs Rust Side-by-Side

```python
# Python                             # Rust
import websockets                    use tungstenite::{accept, Message};

async def echo(ws):                  fn handle(stream: TcpStream) {
    async for msg in ws:                 let mut ws = accept(stream).unwrap();
        await ws.send(msg)               loop {
                                             let msg = ws.read().unwrap();
                                             match msg {
                                                 Message::Text(_) =>
                                                     ws.send(msg).unwrap(),
                                                 Message::Close(_) => break,
                                                 _ => {}
                                             }
                                         }
                                     }
```

---

## 🧪 How to Test Right Now

### Terminal 1 — Start the server:
```bash
cargo run
```

### Terminal 2 — Connect with `wscat` (Node.js WebSocket CLI):
```bash
# Install once
npm install -g wscat

# Connect
wscat -c ws://127.0.0.1:9001
```

Then type any message and press Enter — you'll see it echoed back!

```
Connected (press CTRL+C to quit)
> Hello Rust!
< Hello Rust!
> This is a WebSocket echo server
< This is a WebSocket echo server
```

### Alternative — Test from your browser console:
```javascript
// Open browser DevTools → Console → paste this:
const ws = new WebSocket("ws://127.0.0.1:9001");
ws.onmessage = (e) => console.log("Echo:", e.data);
ws.onopen    = ()  => ws.send("Hello from browser!");
```

---

## ✅ Summary of Step 3

- ✅ `TcpListener::bind()` — same as your previous servers, WebSocket is still TCP underneath
- ✅ `tungstenite::accept(stream)` — performs the HTTP→WebSocket handshake automatically
- ✅ `websocket.read()` — blocks until a message arrives (like Python's `await ws.recv()`)
- ✅ `websocket.send(msg)` — sends a message to the client (like Python's `await ws.send()`)
- ✅ `match message { ... }` — pattern match on the `Message` enum (like Python's `isinstance`)
- ✅ `ref` in match arms — borrow inner data without moving it
- ✅ `message.clone()` — duplicate the message so we can both inspect and send it
- ✅ `match` on `Result` — graceful error handling (like Python's `try/except`)

---

## 🔭 What's Next?

**Step 4** — New Rust concepts deep-dive: `enum`, `match`, `Result`, `loop {}` — all the
patterns you just used, explained thoroughly with Python comparisons.

Then **Step 5** — wire in our `ThreadPool` to handle multiple clients simultaneously!
