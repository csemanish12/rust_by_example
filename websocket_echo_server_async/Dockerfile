# ============================================================================
# Dockerfile — Multi-stage build for websocket_echo_server_async
#
# Stage 1 (builder): Full Rust toolchain — compiles the binary
# Stage 2 (runtime): Tiny Debian image — runs the binary only
#
# Why multi-stage?
#   Rust compiler + dependencies = ~1.5GB image
#   Just the compiled binary    = ~80MB image
#   We build in stage 1, copy only the binary to stage 2
# ============================================================================

# ─────────────────────────────────────────────
# Stage 1: Builder
# ─────────────────────────────────────────────
FROM rust:1.87-slim AS builder

WORKDIR /app

# Step 1: Copy only the manifest files first.
# Docker caches each layer. If Cargo.toml/Cargo.lock haven't changed,
# Docker reuses the cached dependency build — much faster rebuilds.
COPY Cargo.toml Cargo.lock ./

# Step 2: Create a fake main.rs and build dependencies only.
# This pre-compiles all dependencies into a cached layer.
# On next build, if only src/ changed, deps are NOT recompiled.
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

# Step 3: Copy the real source code and build the final binary
COPY src ./src

# Touch main.rs so Cargo knows it changed (avoids stale cache)
RUN touch src/main.rs
RUN cargo build --release

# ─────────────────────────────────────────────
# Stage 2: Runtime
# ─────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install ca-certificates — needed for TLS connections (future-proofing)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy ONLY the compiled binary from the builder stage
# Everything else (Rust toolchain, source code, deps) is left behind
COPY --from=builder /app/target/release/websocket_echo_server_async .

# Expose the WebSocket port
EXPOSE 9001

# Set the binary as the entrypoint
CMD ["./websocket_echo_server_async"]
