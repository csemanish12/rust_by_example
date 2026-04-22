# Step 1 — What is WebSocket? (Async Edition)

> *"Why HTTP falls short, what WebSocket solves, and why async matters for WebSockets specifically."*

---

## 🌐 The Three Protocols

```
Raw TCP       →  just bytes, no rules
     │
     └── HTTP      →  structured request/response, closes after each one
              │
              └── WebSocket  →  persistent, two-way, stays open forever
```

---

## 📄 HTTP — The One-Shot Problem

```
Client                          Server
  │── "GET / HTTP/1.1" ────────►│
  │◄── "200 OK..." ─────────────│
  │         CONNECTION CLOSED   │  ← server can NEVER send again unprompted
```

This is fine for web pages. But for:
- 💬 Chat apps
- 📊 Live dashboards
- 🎮 Multiplayer games

...you need the connection to **stay open**.

---

## 🔌 WebSocket — Persistent Two-Way Connection

```
Client                          Server
  │── HTTP Upgrade request ────►│
  │◄── 101 Switching Protocols ─│   ← handshake (one time only)
  │                             │
  │── "hello" ─────────────────►│   ← client sends anytime
  │◄── "hello" ─────────────────│   ← server echoes back
  │◄── "server push!" ──────────│   ← server sends UNPROMPTED
  │── [Close frame] ───────────►│
```

---

## ⚡ Why Async Matters SPECIFICALLY for WebSocket

This is the key insight for this project:

```
A WebSocket client might be connected for HOURS.
They might send a message once every few minutes.

Sync server:
  Worker thread assigned to Client 1
  → blocks on websocket.read()
  → sits there frozen for 3 minutes waiting
  → can't serve anyone else
  → 4 workers = 4 clients maximum

Async server:
  Task assigned to Client 1
  → calls receiver.next().await
  → YIELDS — "wake me when data arrives"
  → tokio uses this thread for other tasks in the meantime
  → 4 threads = 10,000 clients easily
```

```python
# Python sync — one thread blocked per idle client
while True:
    data = conn.recv(1024)  # 😴 thread frozen here for minutes

# Python async — thread freed while waiting
async for message in websocket:   # ✅ yields to event loop while waiting
    await websocket.send(message)
```

This is exactly why production WebSocket servers use async.

---

## 🆚 Our Two Servers Compared

| | `websocket_echo_server` | `websocket_echo_server_async` |
| :--- | :--- | :--- |
| **Concurrency model** | Thread pool (4 workers) | Async tasks (unlimited) |
| **Max clients** | 4 simultaneous | Thousands |
| **Idle client cost** | Full thread blocked | Near zero |
| **Runtime** | OS threads | `tokio` event loop |
| **Python equivalent** | `ThreadPoolExecutor` | `asyncio` |

---

## ✅ Summary

- ✅ WebSocket = persistent, two-way connection over TCP
- ✅ The handshake starts as HTTP then upgrades (`accept_async()` handles this)
- ✅ Idle WebSocket clients **waste threads** in a sync server
- ✅ Async fixes this — `.await` yields the thread while waiting for messages
- ✅ `tokio` is Rust's async runtime (like Python's `asyncio` event loop)
