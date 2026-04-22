# ── Stage 1: runtime image ────────────────────────────────────────────────────
# Python doesn't need a separate build stage like Rust (no compilation).
# We go straight to the slim runtime image.
FROM python:3.13-slim

WORKDIR /app

# Install dependencies first — Docker layer cache means this layer is only
# rebuilt when requirements.txt changes, not when main.py changes.
# This is the same caching trick used in the Rust multi-stage Dockerfile.
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY main.py .

# Expose the same port as the Rust server for a fair comparison
EXPOSE 9001

# uvicorn is the ASGI server — analogous to the Rust binary itself.
# --workers 1 ensures a single process (same as the Rust server's single tokio runtime).
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "9001", "--workers", "1"]
