// k6 WebSocket load test for websocket_echo_server_async
//
// Usage:
//   k6 run load_test.js                        # default: 100 VUs, 30s
//   k6 run -e SCENARIO=ramp load_test.js       # ramp: 0→500→0 over 2min
//   k6 run -e SCENARIO=spike load_test.js      # spike: sudden 1000 VUs burst
//
// Each virtual user (VU) = one persistent WebSocket connection
// that sends one message per second and checks the echo reply.

import ws from "k6/ws";
import { check, sleep } from "k6";
import { Trend, Counter, Rate } from "k6/metrics";
import { randomString } from "https://jslib.k6.io/k6-utils/1.4.0/index.js";

// ── Custom metrics ────────────────────────────────────────────────────────────
const echoLatency = new Trend("echo_latency_ms", true); // track latency in ms
const messagesSent = new Counter("messages_sent");
const messagesReceived = new Counter("messages_received");
const echoMismatch = new Counter("echo_mismatch"); // data integrity failures
const connectionErrors = new Rate("connection_errors");

// ── Scenarios ─────────────────────────────────────────────────────────────────
const SCENARIO = __ENV.SCENARIO || "steady";

const scenarios = {
  // Hold a fixed number of connections for a sustained period
  steady: {
    executor: "constant-vus",
    vus: 100,
    duration: "30s",
  },

  // Gradually ramp up, hold, then ramp back down
  ramp: {
    executor: "ramping-vus",
    startVUs: 0,
    stages: [
      { duration: "20s", target: 200 },  // ramp up to 200
      { duration: "30s", target: 500 },  // ramp up to 500
      { duration: "30s", target: 500 },  // hold at 500
      { duration: "20s", target: 0 },    // ramp down
    ],
  },

  // Sudden burst — tests how the server handles connection storms
  spike: {
    executor: "ramping-vus",
    startVUs: 0,
    stages: [
      { duration: "10s", target: 5000 }, // spike to 5000
      { duration: "30s", target: 5000 }, // hold
      { duration: "10s", target: 0 },    // drop
    ],
  },
};

export const options = {
  scenarios: {
    websocket_load: scenarios[SCENARIO],
  },

  // Pass/fail thresholds — k6 exits with code 1 if any threshold is breached
  thresholds: {
    // 99% of echo round-trips must be under 1000ms
    echo_latency_ms: ["p(99)<1000"],

    // Less than 1% connection errors allowed
    connection_errors: ["rate<0.01"],

    // No data integrity failures allowed
    echo_mismatch: ["count==0"],
  },
};

// ── Main VU function ───────────────────────────────────────────────────────────
// Each VU runs this function once. Inside, the WebSocket stays open for the
// full scenario duration — sending a message every second.
export default function () {
  const url = __ENV.WS_URL || "ws://localhost:9001";

  // Each connection gets its own pending-message map: payload → sent timestamp
  const pending = new Map();
  let connected = false;

  const res = ws.connect(url, null, function (socket) {
    socket.on("open", function () {
      connected = true;

      // Send one echo per second for the lifetime of this connection
      socket.setInterval(function () {
        const payload = randomString(16); // unique per message
        pending.set(payload, Date.now());
        socket.send(payload);
        messagesSent.add(1);
      }, 1000);
    });

    socket.on("message", function (data) {
      const sentAt = pending.get(data);

      if (sentAt === undefined) {
        // Got a reply we didn't send — data integrity problem
        echoMismatch.add(1);
        return;
      }

      pending.delete(data);
      const latency = Date.now() - sentAt;
      echoLatency.add(latency);
      messagesReceived.add(1);
    });

    socket.on("error", function (e) {
      // k6 surfaces network errors here
      console.error(`WS error: ${e.error()}`);
    });

    socket.on("close", function () {
      // Connection closed cleanly — nothing to do
    });
  });

  // Record whether the initial HTTP→WS upgrade succeeded
  const ok = check(res, {
    "connected successfully": (r) => r && r.status === 101,
  });
  connectionErrors.add(!ok);
}
