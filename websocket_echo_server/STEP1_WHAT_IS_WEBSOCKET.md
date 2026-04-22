# Step 1 — What is WebSocket?

> *"Why not just use HTTP or raw TCP?"*

---

## 🌐 Three Ways to Communicate Over a Network

You've now built two servers:
- `simple_server` → raw **TCP**
- `multithreaded_server` → raw **TCP** (with HTTP response formatting)

WebSocket is the **third** way. Let's understand all three before writing any code.

---

## 📦 Layer 1 — Raw TCP (What You Already Built)

```
Client ──────────────── bytes ──────────────────► Server
       ◄─────────────── bytes ───────────────────
```

- Just a raw **pipe** of bytes between two machines
- No rules about format, no concept of "requests" or "responses"
- You have to invent your own protocol on top
- **Your simple_server spoke HTTP on top of TCP manually**

```python
# Python raw TCP
import socket
s = socket.socket()
s.connect(("127.0.0.1", 7878))
s.send(b"hello")          # raw bytes — no format
data = s.recv(1024)       # raw bytes back
```

```rust
// Rust raw TCP
use std::net::TcpStream;
let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();
stream.write_all(b"hello").unwrap();
```

**Problem:** No structure. You must define every rule yourself.

---

## 📄 Layer 2 — HTTP (Request → Response, then DONE)

HTTP is a **protocol built on top of TCP**. It adds structure:

```
Client                                    Server
  │                                         │
  │──── "GET /hello HTTP/1.1\r\n..." ──────►│
  │                                         │  (processes request)
  │◄─── "HTTP/1.1 200 OK\r\n..." ──────────│
  │                                         │
  │         CONNECTION CLOSED               │  ← this is the key problem
```

### The HTTP Problem: It's One-Shot

Every HTTP interaction is:
1. Client connects
2. Client sends ONE request
3. Server sends ONE response
4. **Connection closes** ← server can't send more data later!

```python
# Python HTTP — server can NEVER push data to client unprompted
import requests
r = requests.get("http://127.0.0.1:7878")  # one request
print(r.text)                               # one response — done
# Server cannot send you anything after this
```

**Real-world problem:** How do you build a chat app, live dashboard, or multiplayer game with HTTP?
- ❌ Long-polling (client keeps asking "anything new?") — wasteful
- ❌ Server-Sent Events — one direction only
- ✅ **WebSocket** — solves all of this

---

## 🔌 Layer 3 — WebSocket (Persistent Two-Way Connection)

```
Client                                    Server
  │                                         │
  │──── HTTP Upgrade Request ──────────────►│   ← "handshake"
  │◄─── 101 Switching Protocols ────────────│   ← server agrees
  │                                         │
  │         CONNECTION STAYS OPEN  ─────────│── forever (or until closed)
  │                                         │
  │──── "Hello!" ──────────────────────────►│   ← client sends anytime
  │◄─── "Hello!" ───────────────────────────│   ← server echoes back
  │                                         │
  │◄─── "Server push!" ─────────────────────│   ← server sends UNPROMPTED
  │                                         │
  │──── "Bye" ─────────────────────────────►│
  │◄─── [Close frame] ──────────────────────│
```

### Key Properties

| Property | HTTP | WebSocket |
| :--- | :--- | :--- |
| **Connection** | Opens → closes per request | Stays open permanently |
| **Direction** | Client → Server only | **Both directions** (full-duplex) |
| **Server push** | ❌ Not possible | ✅ Server can send anytime |
| **Overhead** | Headers on every request | Headers only on handshake |
| **Use cases** | Web pages, REST APIs | Chat, live feeds, games |
| **Built on** | TCP | **Also TCP** (starts as HTTP!) |

---

## 🤝 The WebSocket Handshake — How It Starts

WebSocket is clever — it **starts as HTTP**, then upgrades:

```
Step 1: Client sends a normal-looking HTTP request:

  GET / HTTP/1.1
  Host: 127.0.0.1:9001
  Upgrade: websocket              ← "I want to switch to WebSocket"
  Connection: Upgrade
  Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==   ← random key
  Sec-WebSocket-Version: 13

Step 2: Server responds:

  HTTP/1.1 101 Switching Protocols
  Upgrade: websocket
  Connection: Upgrade
  Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=  ← derived from key

Step 3: Both sides switch to the WebSocket frame format.
        HTTP is never used again on this connection.
```

The good news: **`tungstenite` handles ALL of this for you** with one function call:
```rust
let websocket = tungstenite::accept(tcp_stream).unwrap();
//                              ☝️ does the entire handshake automatically
```

```python
# Python equivalent with the 'websockets' library
import asyncio, websockets

async def handler(websocket):
    # tungstenite.accept() ≈ websockets.serve() — handshake is automatic
    async for message in websocket:
        await websocket.send(message)  # echo it back
```

---

## 📨 WebSocket Messages — Not Raw Bytes Anymore

Unlike TCP (raw bytes), WebSocket gives you **structured messages**:

```
WebSocket Frame:
┌─────────┬──────────┬─────────────────────────────────────┐
│  flags  │  opcode  │              payload                 │
│  (1B)   │  (4bit)  │           (variable length)         │
└─────────┴──────────┴─────────────────────────────────────┘
         ▲
         └── opcode tells you WHAT TYPE of message this is:
               0x1 = Text   (a UTF-8 string)
               0x2 = Binary (raw bytes)
               0x8 = Close  (end the connection)
               0x9 = Ping   (keepalive check)
               0xA = Pong   (response to ping)
```

In Rust's `tungstenite`, these opcodes are represented as an **enum** (more on this in Step 4):

```rust
// tungstenite::Message — the enum you'll pattern-match on
match message {
    Message::Text(text)     => { /* a text string */ }
    Message::Binary(bytes)  => { /* raw bytes     */ }
    Message::Close(_)       => { /* client closed */ }
    Message::Ping(data)     => { /* keepalive     */ }
    Message::Pong(data)     => { /* ping response */ }
    _                       => { /* ignore others */ }
}
```

```python
# Python websockets library — similar concept
async def handler(ws):
    async for message in ws:
        if isinstance(message, str):      # Text
            await ws.send(message)
        elif isinstance(message, bytes):  # Binary
            await ws.send(message)
        # Close/Ping/Pong handled automatically by the library
```

---

## 🆚 WebSocket vs What You've Built

| Project | Protocol | Connection | Direction | Use Case |
| :--- | :--- | :--- | :--- | :--- |
| `simple_server` | HTTP over TCP | Per-request | Client→Server | Web pages |
| `multithreaded_server` | HTTP over TCP | Per-request | Client→Server | Web pages (faster) |
| `websocket_echo_server` | **WebSocket over TCP** | **Persistent** | **Both ways** | Chat, live apps |

---

## 🐍 Python vs Rust — The Libraries

| | Python | Rust |
| :--- | :--- | :--- |
| **Library** | `websockets` (async) or `websocket-client` | `tungstenite` (sync) |
| **Install** | `pip install websockets` | `cargo add tungstenite` |
| **Accept connection** | `websockets.serve(handler, host, port)` | `tungstenite::accept(tcp_stream)` |
| **Read message** | `await websocket.recv()` | `websocket.read().unwrap()` |
| **Send message** | `await websocket.send(msg)` | `websocket.send(Message::Text(msg))` |

> **Key difference:** Python's `websockets` is **async** (uses `asyncio`).
> `tungstenite` is **synchronous** — same mental model as your TCP servers.
> This makes it much easier to learn first!

---

## ✅ Summary of Step 1

- ✅ **TCP** = raw byte pipe — no structure, you invent the rules
- ✅ **HTTP** = structured request/response on top of TCP — but closes after each exchange
- ✅ **WebSocket** = persistent, two-way, full-duplex connection — starts as HTTP then upgrades
- ✅ The **handshake** is automatic — `tungstenite::accept()` handles it
- ✅ Messages come in **typed frames** — `Text`, `Binary`, `Ping`, `Close`
- ✅ `tungstenite` = the Rust library we'll use (sync, beginner-friendly)

---

## 🔭 What's Next?

**Step 2** — Adding `tungstenite` as a dependency using `Cargo.toml`.
We'll learn how Rust manages external libraries compared to Python's `pip` + `requirements.txt`.
