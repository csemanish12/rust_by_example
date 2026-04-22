"""
smoke_test.py — Correctness + basic concurrency test for the async WebSocket server.

What this tests:
  1. Can N clients connect simultaneously?
  2. Does every client get its message echoed back correctly?
  3. How long does it take for all N clients to complete?

Usage:
  pip install -r requirements.txt
  python smoke_test.py                  # default: 10 clients
  python smoke_test.py --clients 100
  python smoke_test.py --clients 500 --url ws://localhost:9001

Run the server first:
  docker compose up -d      (Docker)
  cargo run                 (local)
"""

import asyncio
import argparse
import time
import sys
from dataclasses import dataclass, field

import websockets


# ─────────────────────────────────────────────
# Result tracking
# ─────────────────────────────────────────────

@dataclass
class ClientResult:
    client_id: int
    success: bool
    latency_ms: float        # round-trip time for one echo
    error: str = ""


# ─────────────────────────────────────────────
# Single client — connect, send, receive, verify
# ─────────────────────────────────────────────

async def run_client(client_id: int, url: str, timeout: float) -> ClientResult:
    message = f"hello from client {client_id}"
    try:
        async with websockets.connect(url, open_timeout=timeout) as ws:
            # Send a message and measure round-trip time
            t0 = time.perf_counter()
            await ws.send(message)
            echo = await asyncio.wait_for(ws.recv(), timeout=timeout)
            latency_ms = (time.perf_counter() - t0) * 1000

            # Verify the echo is correct
            if echo != message:
                return ClientResult(
                    client_id=client_id,
                    success=False,
                    latency_ms=latency_ms,
                    error=f"Echo mismatch! Sent: '{message}' Got: '{echo}'"
                )

            return ClientResult(client_id=client_id, success=True, latency_ms=latency_ms)

    except asyncio.TimeoutError:
        return ClientResult(client_id=client_id, success=False, latency_ms=0, error="Timeout")
    except Exception as e:
        return ClientResult(client_id=client_id, success=False, latency_ms=0, error=str(e))


# ─────────────────────────────────────────────
# Main — run N clients concurrently
# ─────────────────────────────────────────────

async def main(num_clients: int, url: str, timeout: float):
    print(f"\n{'='*60}")
    print(f"  WebSocket Echo Server — Smoke Test")
    print(f"{'='*60}")
    print(f"  URL          : {url}")
    print(f"  Clients      : {num_clients}")
    print(f"  Timeout      : {timeout}s per client")
    print(f"{'='*60}\n")

    # Launch all clients at the same time
    wall_start = time.perf_counter()

    tasks = [run_client(i, url, timeout) for i in range(num_clients)]
    results: list[ClientResult] = await asyncio.gather(*tasks)

    wall_elapsed = (time.perf_counter() - wall_start) * 1000  # ms

    # ── Analyse results ──────────────────────────────────────
    passed   = [r for r in results if r.success]
    failed   = [r for r in results if not r.success]
    latencies = sorted(r.latency_ms for r in passed)

    def percentile(data: list[float], p: int) -> float:
        if not data:
            return 0.0
        idx = int(len(data) * p / 100)
        return data[min(idx, len(data) - 1)]

    # ── Print results ─────────────────────────────────────────
    print(f"  Results")
    print(f"  {'─'*40}")
    print(f"  Total clients  : {num_clients}")
    print(f"  ✅  Passed      : {len(passed)}")
    print(f"  ❌  Failed      : {len(failed)}")
    print(f"  Success rate   : {len(passed)/num_clients*100:.1f}%")
    print(f"  Wall time      : {wall_elapsed:.1f} ms")
    print()

    if latencies:
        print(f"  Latency (echo round-trip)")
        print(f"  {'─'*40}")
        print(f"  min    : {latencies[0]:.2f} ms")
        print(f"  p50    : {percentile(latencies, 50):.2f} ms")
        print(f"  p90    : {percentile(latencies, 90):.2f} ms")
        print(f"  p95    : {percentile(latencies, 95):.2f} ms")
        print(f"  p99    : {percentile(latencies, 99):.2f} ms")
        print(f"  max    : {latencies[-1]:.2f} ms")

    if failed:
        print(f"\n  Failures")
        print(f"  {'─'*40}")
        # Show first 10 failures only
        for r in failed[:10]:
            print(f"  Client {r.client_id:>4}: {r.error}")
        if len(failed) > 10:
            print(f"  ... and {len(failed) - 10} more")

    print(f"\n{'='*60}\n")

    # Exit with non-zero code if any failures — useful for CI
    if failed:
        sys.exit(1)


# ─────────────────────────────────────────────
# CLI
# ─────────────────────────────────────────────

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="WebSocket echo server smoke test")
    parser.add_argument("--clients", type=int, default=10, help="Number of concurrent clients (default: 10)")
    parser.add_argument("--url",     type=str, default="ws://localhost:9001", help="WebSocket server URL")
    parser.add_argument("--timeout", type=float, default=10.0, help="Per-client timeout in seconds (default: 10)")
    args = parser.parse_args()

    asyncio.run(main(args.clients, args.url, args.timeout))
