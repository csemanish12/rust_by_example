# Load Testing — `websocket_echo_server_async`

This document covers the full testing strategy for the async WebSocket server:
correctness first, then sustained load, then connection storms.

---

## The Two Tools

| Tool | What it tests | How |
|------|---------------|-----|
| **Python smoke test** | Correctness — did every client get its message back? | Burst: N clients connect, send one message, disconnect |
| **k6 load test** | Sustained performance — latency, throughput, stability | Persistent connections open for the full test duration, sending 1 msg/sec each |

They answer different questions. The smoke test proves the server is **correct**.  
k6 proves it stays correct and fast **under pressure over time**.

---

## Architecture Under Test

```
k6 / Python
    │
    │  WebSocket (ws://)
    ▼
┌─────────────────────────────┐
│  Docker container           │
│  1 CPU core · 256 MB RAM    │
│                             │
│  websocket_echo_server_async│
│  tokio runtime              │
│  └── task per connection    │
└─────────────────────────────┘
```

The server runs inside Docker with hard resource limits:

```yaml
# docker-compose.yml
deploy:
  resources:
    limits:
      cpus: "1.0"
      memory: 256m
```

Everything below happened on **1 CPU core**.

---

## Step 1 — Smoke Test (Python / asyncio)

**File:** `tests/python/smoke_test.py`

**What it does:**  
Spawns N concurrent asyncio tasks. Each task:
1. Opens a WebSocket connection
2. Sends one unique message
3. Waits for the echo reply
4. Measures round-trip latency
5. Disconnects

All N tasks run concurrently via `asyncio.gather()` — same idea as `tokio::join_all` in Rust.

**Run it:**
```bash
cd tests/python
python3 -m venv venv && source venv/bin/activate
pip install -r requirements.txt

python smoke_test.py --clients 100
python smoke_test.py --clients 500
python smoke_test.py --clients 1000
```

### Results

| Clients | Success | p50 | p99 | Wall time |
|---------|---------|-----|-----|-----------|
| 10 | 100% | ~1ms | 1.7ms | 569ms |
| 50 | 100% | ~5ms | 10ms | 62ms |
| 100 | 100% | ~5ms | 10ms | 87ms |
| 500 | 100% | 31ms | 70ms | 1.17s |
| 1000 | 100% | 46ms | 176ms | 561ms |

**Zero failures at every level.** Latency grows with client count because all 1000 connections share one CPU core — that's expected, not a bug.

---

## Step 2 — k6 Load Test (Persistent Connections)

**File:** `tests/k6/load_test.js`

**What it does:**  
Each k6 virtual user (VU) = one persistent WebSocket connection that stays open for the entire test. Every second, each VU:
1. Sends a random 16-character string
2. Waits for the echo reply
3. Records the round-trip latency

This is a more realistic model of real WebSocket usage (chat, live dashboards, game clients) where connections are long-lived.

**Custom metrics tracked:**

| Metric | Type | Description |
|--------|------|-------------|
| `echo_latency_ms` | Trend | Round-trip time per message |
| `messages_sent` | Counter | Total messages sent |
| `messages_received` | Counter | Total messages received |
| `echo_mismatch` | Counter | Replies that didn't match the sent payload |
| `connection_errors` | Rate | % of connections that failed to upgrade |

**Thresholds (CI pass/fail gates):**
```
echo_latency_ms  p(99) < 300ms   ← 99% of echoes under 300ms
connection_errors rate < 1%       ← less than 1% failed connections
echo_mismatch     count == 0      ← zero data integrity failures
```

---

## Scenarios

### 1. Steady — `k6 run load_test.js`

100 connections held open for 30 seconds.

```
100 VUs ──────────────────────────── 30s
```

**Results:**

```
echo_latency_ms : avg=4ms  p(90)=7ms  p(95)=8ms  p(99)=10ms
messages_sent   : 5,900
connection_errors: 0%
echo_mismatch   : 0
ws_connecting   : avg=24ms  p(95)=37ms
```

---

### 2. Ramp — `k6 run -e SCENARIO=ramp load_test.js`

Gradual ramp from 0 → 200 → 500, hold, ramp back to 0. Tests how the server handles a growing connection pool.

```
VUs
500 │          ┌───────────┐
400 │         /│           │\
300 │        / │           │ \
200 │       /  │           │  \
100 │      /   │           │   \
  0 └─────┴───────────────────────── time
      20s  20s     30s      20s
```

**Results:**

```
echo_latency_ms : avg=4ms  p(90)=7ms  p(95)=8ms  p(99)=10ms
messages_sent   : 47,230
connection_errors: 0%
echo_mismatch   : 0
ws_connecting   : avg=5ms  p(95)=10ms
```

p99 held at **10ms** even at 500 connections. Connection establishment
got *faster* vs the steady scenario — at lower concurrency early in the
ramp, the server had more headroom.

---

### 3. Spike — `k6 run -e SCENARIO=spike load_test.js`

1000 connections in 5 seconds, held for 20 seconds, then dropped. Tests
the server's ability to handle a connection storm without dropping messages.

```
VUs
1000 │    ┌──────────────────┐
 500 │   /│                  │\
   0 └──┴─┴──────────────────┴─── time
      5s        20s          5s
```

**Results:**

```
echo_latency_ms : avg=1ms  p(90)=2ms  p(95)=3ms  p(99)=8ms
messages_sent   : 54,499
messages_received: 54,499  ← every single one matched
connection_errors: 0%
echo_mismatch   : 0
ws_connecting   : avg=1.7ms  p(95)=2.6ms
```

**p99 actually improved to 8ms vs the smoke test's 176ms at 1000 clients.**  
The difference: k6 VUs send only 1 message/sec (spread load), while the
smoke test fires all messages simultaneously (burst load).

---

## All Thresholds — Every Run

| Scenario | `p(99)<300ms` | `connection_errors<1%` | `echo_mismatch==0` |
|----------|--------------|------------------------|-------------------|
| Steady (100 VUs) | ✅ 10ms | ✅ 0% | ✅ |
| Ramp (0→500) | ✅ 10ms | ✅ 0% | ✅ |
| Spike (1000 VUs) | ✅ 8ms | ✅ 0% | ✅ |

---

## Why the Numbers Are What They Are

### `echo_mismatch = 0` always
Each connection runs in its own `tokio::spawn`ed task with its own
`SplitSink` / `SplitStream`. There is no shared message state between connections.
A message sent on connection A can only come back on connection A.

```rust
// Each task owns its sender and receiver — no sharing
let (sender, receiver) = ws_stream.split();
tokio::spawn(handle_client(sender, receiver));
```

### Latency flat across 100→500 connections
Tokio tasks are cooperative — an idle connection (waiting for the next
timer tick) costs almost nothing. The CPU is only used when a message
actually arrives. 500 connections sending 1 msg/sec = 500 events/sec,
which is trivial for a modern async runtime.

### Latency rises sharply from 100→1000 in burst mode
The smoke test fires all N messages at the same instant. At 1000 clients
that's 1000 simultaneous syscalls competing for 1 CPU core. tokio still
handles all of them — just takes ~176ms p99 instead of ~10ms.

### Connection upgrade (ws_connecting) p95 < 3ms at 1000 VUs
`tokio::spawn` returns immediately — accepting a new connection doesn't
block existing ones. Each connection upgrade is an independent async task.

---

## Python vs Rust — Testing Parallels

| Python (smoke test) | Rust (server internals) |
|---------------------|------------------------|
| `asyncio.gather()` | `tokio::join_all()` |
| `async def client()` | `async fn handle_client()` |
| `await websocket.send()` | `sender.send().await` |
| `asyncio.wait_for(..., timeout)` | `tokio::time::timeout()` |
| One event loop, many coroutines | One tokio runtime, many tasks |

Both are **single-threaded concurrent** models. Neither uses threads for
concurrency — they use cooperative scheduling on a single thread (or small
thread pool in tokio's case).

---

## Running Everything

Make sure the Docker server is running first:

```bash
# From websocket_echo_server_async/
docker compose up -d
docker compose ps   # should show "running"
```

Then run tests:

```bash
# Smoke tests
cd tests/python
source venv/bin/activate
python smoke_test.py --clients 100
python smoke_test.py --clients 1000

# k6 load tests
cd tests/k6
k6 run load_test.js                      # steady: 100 VUs, 30s
k6 run -e SCENARIO=ramp load_test.js     # ramp: 0→500
k6 run -e SCENARIO=spike load_test.js    # spike: 1000 VUs burst
```

To stop the server:
```bash
docker compose down
```
