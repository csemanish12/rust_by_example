# Step 6 — Testing the WebSocket Echo Server

> *"Verify everything works — interactively, from the browser, and under load."*

---

## 🧰 Tools We'll Use

| Tool | What it is | Python equivalent |
| :--- | :--- | :--- |
| `wscat` | CLI WebSocket client | `python -m websockets` / `websocat` |
| Browser DevTools | Built-in JS WebSocket API | N/A |
| `curl` (HTTP check) | Verify the port is open | `requests.get(...)` |

---

## 🚀 Start the Server

```bash
cargo run
```

Expected output:
```
🚀 WebSocket Echo Server listening on ws://127.0.0.1:9001
   Workers : 4 (fixed thread pool)
   Test with: wscat -c ws://127.0.0.1:9001
```

---

## 🧪 Test 1 — Interactive Echo with `wscat`

### Install `wscat` (one time):
```bash
npm install -g wscat
```

### Connect and chat:
```bash
wscat -c ws://127.0.0.1:9001
```

```
Connected (press CTRL+C to quit)
> Hello Rust!
< Hello Rust!
> WebSocket is fun
< WebSocket is fun
> 42
< 42
> (CTRL+C to disconnect)
Disconnected
```

### What you'll see in the server logs:
```
[127.0.0.1:54321] TCP connection → sending to thread pool...
[Worker 0] picked up a WebSocket client.
[127.0.0.1:54321] ✅ WebSocket handshake complete!
[127.0.0.1:54321] 📨 "Hello Rust!"
[127.0.0.1:54321] 📤 Echoed.
[127.0.0.1:54321] 📨 "WebSocket is fun"
[127.0.0.1:54321] 📤 Echoed.
[127.0.0.1:54321] Disconnected: ...
[127.0.0.1:54321] Connection closed. Worker is now free.
```

---

## 🧪 Test 2 — From the Browser Console

No extra tools needed — every browser has a WebSocket API built in.

1. Open any browser (Chrome, Firefox, Safari)
2. Open **DevTools** → **Console** tab (`F12` or `Cmd+Option+I`)
3. Paste this:

```javascript
// Open a WebSocket connection to our Rust server
const ws = new WebSocket("ws://127.0.0.1:9001");

// When connection is open — send a message
ws.onopen = () => {
    console.log("✅ Connected to Rust WebSocket server!");
    ws.send("Hello from the browser!");
};

// When we receive an echo back
ws.onmessage = (event) => {
    console.log("📨 Echo received:", event.data);
};

// When connection closes
ws.onclose = () => console.log("🔌 Disconnected.");

// When there's an error
ws.onerror = (e) => console.error("❌ Error:", e);
```

4. You'll see in the browser console:
```
✅ Connected to Rust WebSocket server!
📨 Echo received: Hello from the browser!
```

5. Send more messages anytime:
```javascript
ws.send("Another message!");
// → 📨 Echo received: Another message!
```

6. Close cleanly:
```javascript
ws.close();
// → 🔌 Disconnected.
```

---

## 🧪 Test 3 — Multiple Clients at Once (Thread Pool Proof)

This is the key test — proving that 4 workers handle 4 clients **simultaneously**.

### Open 5 separate terminal tabs and run in each:
```bash
wscat -c ws://127.0.0.1:9001
```

### Server logs — what to look for:

```
[127.0.0.1:54001] TCP connection → sending to thread pool...
[Worker 0] picked up a WebSocket client.    ← Worker 0 busy

[127.0.0.1:54002] TCP connection → sending to thread pool...
[Worker 1] picked up a WebSocket client.    ← Worker 1 busy

[127.0.0.1:54003] TCP connection → sending to thread pool...
[Worker 2] picked up a WebSocket client.    ← Worker 2 busy

[127.0.0.1:54004] TCP connection → sending to thread pool...
[Worker 3] picked up a WebSocket client.    ← Worker 3 busy

[127.0.0.1:54005] TCP connection → sending to thread pool...
← Client 5 is QUEUED — no free workers yet

[127.0.0.1:54001] Connection closed. Worker is now free.
[Worker 0] picked up a WebSocket client.    ← Worker 0 picks up Client 5!
```

### ✅ What this proves:
- Only **4 threads ever exist** — no matter how many clients connect
- Client 5 **waits** in the channel queue (not dropped, not errored)
- As soon as any worker finishes, it **immediately** picks up the next client
- The main loop never blocks — it keeps accepting connections freely

---

## 🧪 Test 4 — Binary Messages from the Browser

```javascript
// Send a binary message (like an image or file chunk)
const data = new Uint8Array([72, 101, 108, 108, 111]); // "Hello" in bytes
ws.send(data.buffer);

// Server will log:
// [127.0.0.1:54321] 📦 5 bytes
// [127.0.0.1:54321] 📤 Echoed.

// And you'll receive the binary echo:
ws.onmessage = (event) => {
    if (event.data instanceof ArrayBuffer) {
        const view = new Uint8Array(event.data);
        console.log("📦 Binary echo:", view);
        // → 📦 Binary echo: Uint8Array(5) [72, 101, 108, 108, 111]
    }
};
```

---

## 🧪 Test 5 — Graceful Close

Test that the server handles the Close frame properly (not a crash):

```bash
wscat -c ws://127.0.0.1:9001
Connected
> hello
< hello
> (type CTRL+C)   ← abrupt disconnect
```

vs

```javascript
// Browser: clean close
ws.close(1000, "Done testing");
// Server logs:
// [127.0.0.1:54321] 🔌 Close frame received. Goodbye!
```

| Disconnect type | Server sees |
| :--- | :--- |
| `ws.close()` — clean | `Message::Close` → sends close frame back → `break` |
| CTRL+C / tab close | `Err(ConnectionReset)` → logs error → `break` |
| Network drop | `Err(...)` → logs error → `break` |

All three cases are handled — the server **never crashes** on a client disconnect.

---

## 🧪 Test 6 — Verify the Thread Pool Limit with a Script

Run this in a terminal to open 10 connections at once:

```bash
for i in {1..10}; do
    wscat -c ws://127.0.0.1:9001 --execute "test message $i" &
done
wait
```

Watch the server — you'll never see more than 4 `[Worker X] picked up` lines active at the same time.

---

## 📊 What Good Output Looks Like

```
🚀 WebSocket Echo Server listening on ws://127.0.0.1:9001
   Workers : 4 (fixed thread pool)

[127.0.0.1:54001] TCP connection → sending to thread pool...
[Worker 0] picked up a WebSocket client.
[127.0.0.1:54001] ✅ WebSocket handshake complete!
[127.0.0.1:54001] 📨 "hello"
[127.0.0.1:54001] 📤 Echoed.
[127.0.0.1:54001] 🔌 Close frame received. Goodbye!
[127.0.0.1:54001] Connection closed. Worker is now free.
```

---

## ❌ Common Issues & Fixes

| Error | Cause | Fix |
| :--- | :--- | :--- |
| `Connection refused` | Server not running | `cargo run` first |
| `wscat: command not found` | Not installed | `npm install -g wscat` |
| `WebSocket is closed before the connection is established` | Wrong port | Use port `9001` not `7878` |
| `error: Address already in use` | Old server still running | `pkill websocket_echo_server` |
| Compiler warning: `field id is never read` | Harmless | Add `#[allow(dead_code)]` to `Worker` struct |

---

## ✅ Summary of Step 6 — Checklist

- [ ] `cargo run` starts without errors
- [ ] `wscat` connects and echoes text messages back
- [ ] Browser DevTools WebSocket test works
- [ ] 4 simultaneous clients handled by 4 different workers
- [ ] 5th client waits and is picked up when a worker is free
- [ ] Clean close (`ws.close()`) shows `🔌 Close frame received`
- [ ] Abrupt disconnect shows `Disconnected:` error log (not a crash)

---

## 🎉 Project Complete!

You've built a production-safe, multi-client WebSocket echo server in Rust — from scratch.

### What You've Learned Across All Projects:

```
simple_server          → TCP, HTTP, Ownership, Drop
multithreaded_server   → Threads, Arc, Mutex, Channels, ThreadPool
websocket_echo_server  → WebSocket, tungstenite, enum, match, Result, Crates
```

### The Rust Skills You Now Have:

| Concept | Where You Used It |
| :--- | :--- |
| Ownership & `move` | Every project |
| `match` + `Result` | WebSocket message handling |
| `enum` | `Message` variants |
| `Arc<Mutex<T>>` | Shared receiver in ThreadPool |
| `mpsc::channel` | Job queue between pool and workers |
| `Box<dyn FnOnce()>` | The `Job` type |
| `Drop` trait | Graceful pool shutdown |
| External crates | `tungstenite` via `Cargo.toml` |
| `loop` + `break` | Echo loop, worker loop |

**Next ideas to extend this project:**
- Broadcast messages to ALL connected clients (needs `Arc<Mutex<Vec<WebSocket>>>`)
- Add a `/ping` HTTP health check endpoint
- Build a simple chat room on top of the echo server
- Add TLS support (`tungstenite` has a TLS feature flag)
