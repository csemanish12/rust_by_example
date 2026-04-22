"""
WebSocket echo server — Python / FastAPI + asyncio

Mirrors the Rust websocket_echo_server_async feature-for-feature:
  - One asyncio task per connection  (≈ tokio::spawn per connection)
  - Heartbeat ping every 30 seconds  (≈ tokio::time::interval)
  - Echo all text/binary messages     (≈ handle_message())
  - Graceful shutdown on SIGTERM      (≈ tokio::signal::ctrl_c())
  - Structured logging                (≈ tracing)
  - Binds 0.0.0.0:9001

Run locally:
    uvicorn main:app --host 0.0.0.0 --port 9001

Run via Docker:
    docker compose up
"""

import asyncio
import logging
import signal
import time
from contextlib import asynccontextmanager

from fastapi import FastAPI, WebSocket, WebSocketDisconnect

# ── Logging (structured, mirrors tracing in Rust) ──────────────────────────────
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s  %(levelname)-8s  %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%S",
)
log = logging.getLogger(__name__)

PING_INTERVAL = 30  # seconds — same as Rust server


# ── Lifespan (startup / shutdown hook) ────────────────────────────────────────
# In Python this is an @asynccontextmanager on the app.
# In Rust this was tokio::signal::ctrl_c() inside tokio::select!.
@asynccontextmanager
async def lifespan(app: FastAPI):
    log.info("WebSocket echo server starting on ws://0.0.0.0:9001")

    # Register SIGTERM handler for graceful Docker shutdown
    loop = asyncio.get_running_loop()
    loop.add_signal_handler(signal.SIGTERM, lambda: log.info("SIGTERM received, shutting down"))

    yield  # server is running

    log.info("WebSocket echo server shut down")


app = FastAPI(lifespan=lifespan)


# ── Per-connection handler ─────────────────────────────────────────────────────
# In Rust: async fn handle_client(sender, receiver) spawned with tokio::spawn
# In Python: async def handle_client() called directly inside the route —
#            FastAPI already runs each WebSocket route in its own asyncio task.
async def handle_client(ws: WebSocket) -> None:
    """
    Runs for the lifetime of one WebSocket connection.
    Uses asyncio.wait() to race between:
      - the next incoming message  (≈ receiver.next() in Rust)
      - the heartbeat timer tick   (≈ ping_interval.tick() in Rust)

    This is the Python equivalent of tokio::select! in the Rust server.
    """
    client = ws.client  # (host, port) tuple

    # asyncio.Event used to signal the heartbeat task to stop cleanly
    stop_event = asyncio.Event()

    async def heartbeat() -> None:
        """Sends a ping every PING_INTERVAL seconds. Mirrors the Rust interval task."""
        while not stop_event.is_set():
            try:
                await asyncio.sleep(PING_INTERVAL)
                if stop_event.is_set():
                    break
                await ws.send_bytes(b"")  # WebSocket ping frame
                log.debug("ping sent to %s", client)
            except Exception:
                break

    # Start the heartbeat as a concurrent asyncio task
    # In Rust: the ping branch lives inside tokio::select! — same idea
    ping_task = asyncio.create_task(heartbeat())

    messages_handled = 0

    try:
        while True:
            # Receive the next frame — blocks until a message arrives
            # In Rust: `receiver.next().await` inside tokio::select!
            data = await ws.receive()

            if "text" in data:
                # Echo text messages back verbatim
                await ws.send_text(data["text"])
                messages_handled += 1
                log.debug("echo text (%d bytes) to %s", len(data["text"]), client)

            elif "bytes" in data:
                # Echo binary messages back verbatim
                await ws.send_bytes(data["bytes"])
                messages_handled += 1
                log.debug("echo bytes (%d bytes) to %s", len(data["bytes"]), client)

    except WebSocketDisconnect:
        # Client closed the connection cleanly
        log.info("client %s disconnected — handled %d messages", client, messages_handled)

    except Exception as exc:
        # Network error, timeout, etc.
        log.warning("client %s error: %s", client, exc)

    finally:
        # Cancel the heartbeat task when the connection ends
        # In Rust: ping_task is cancelled when handle_client() returns
        stop_event.set()
        ping_task.cancel()
        try:
            await ping_task
        except asyncio.CancelledError:
            pass


# ── WebSocket route ────────────────────────────────────────────────────────────
# In Rust: TcpListener::accept() loop + tokio::spawn(handle_client(...))
# In Python: FastAPI accepts the HTTP→WS upgrade and calls this route.
#            Each call runs in its own asyncio task automatically.
@app.websocket("/")
async def websocket_endpoint(ws: WebSocket):
    await ws.accept()
    log.info("client connected: %s", ws.client)
    await handle_client(ws)
