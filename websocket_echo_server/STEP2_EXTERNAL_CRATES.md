# Step 2 — External Crates (`Cargo.toml` vs `pip` + `requirements.txt`)

> *"How Rust manages dependencies — and why it's more powerful than pip"*

---

## 🐍 What You Already Know — Python's Way

In Python, to use an external library you:

```bash
# 1. Install it from PyPI
pip install websockets

# 2. (Optionally) save it to a file
pip freeze > requirements.txt

# 3. Use it in code
import websockets
```

**The problem with pip:**
- `requirements.txt` only lists direct dependencies — not their dependencies
- Two machines can install different versions of sub-dependencies
- No guarantee the build is reproducible

---

## 📦 Rust's Way — `Cargo.toml` + `Cargo.lock`

Rust uses **Cargo** as its package manager. External libraries are called **crates**.

```
Python                          Rust
──────                          ────
PyPI              ←→            crates.io       (the package registry)
pip               ←→            cargo           (the package manager)
requirements.txt  ←→            Cargo.toml      (what you want)
pip freeze        ←→            Cargo.lock      (exact versions locked)
import websockets ←→            use tungstenite (bring into scope)
```

---

## 🗂️ The Two Files: `Cargo.toml` vs `Cargo.lock`

### `Cargo.toml` — What YOU write (like `requirements.txt`)

```toml
[package]
name    = "websocket_echo_server"
version = "0.1.0"
edition = "2024"

[dependencies]
tungstenite = "0.26"   # ← you specify this
#             ─────
#             version requirement (semver)
#             "0.26" means "0.26.x — any patch version"
```

You write this **by hand** (or use `cargo add`). It expresses your **intent**.

### `Cargo.lock` — What Cargo writes (like `pip freeze`)

```toml
# Cargo.lock (auto-generated — never edit this manually)
[[package]]
name    = "tungstenite"
version = "0.26.2"          ← exact version locked
source  = "registry+..."
checksum = "abc123..."       ← cryptographic hash — guarantees exact bytes
dependencies = [
  "byteorder 1.5.0",
  "httparse 1.9.5",
  ...                        ← ALL transitive dependencies, exactly pinned
]
```

`Cargo.lock` guarantees **100% reproducible builds** — any developer, any machine,
any CI server gets **exactly** the same binary.

---

## 🔢 Semantic Versioning (Semver) — Reading Version Numbers

```
tungstenite = "0.26"
              ─┬──
               └── This is a semver requirement

Version format:   MAJOR . MINOR . PATCH
                    0   .  26   .  2

"0.26"   means:  >=0.26.0, <0.27.0   (any patch update is fine)
"^0.26"  means:  same as above        (^ is the default)
"=0.26.2" means: exactly 0.26.2 only
"*"      means:  any version          (dangerous — avoid)
```

```python
# Python's pip equivalent version specifiers
websockets>=11.0,<12.0   # pip — explicit range
websockets~=11.0          # pip — same as Cargo's "11.0"
websockets==11.0.3        # pip — exact version
```

---

## ⚡ How to Add a Dependency — Two Ways

### Way 1: Edit `Cargo.toml` directly (what we did)

```toml
[dependencies]
tungstenite = "0.26"
```

### Way 2: Use `cargo add` (like `pip install`)

```bash
cargo add tungstenite
# ☝️ automatically finds the latest version and adds it to Cargo.toml
```

```python
# Python equivalent
pip install websockets
pip freeze > requirements.txt
```

---

## 🏗️ The Full Cargo Workflow

```
You write:          Cargo reads:        Cargo produces:
──────────          ────────────        ───────────────
Cargo.toml   ──►   crates.io    ──►    Cargo.lock   (locked versions)
                                  ──►   target/      (compiled binaries)


cargo build       → download deps + compile everything
cargo run         → build + run
cargo add <crate> → add to Cargo.toml + update Cargo.lock
cargo update      → update Cargo.lock to latest allowed versions
```

```python
# Python equivalent workflow
pip install -r requirements.txt   ≈   cargo build
python main.py                    ≈   cargo run
pip install websockets            ≈   cargo add tungstenite
pip install --upgrade ...         ≈   cargo update
```

---

## 📚 `crates.io` vs `PyPI` — Finding Libraries

| | Python (PyPI) | Rust (crates.io) |
| :--- | :--- | :--- |
| **Website** | pypi.org | crates.io |
| **Docs** | varies | **docs.rs** (automatic for ALL crates) |
| **Search** | pypi.org/search | crates.io/search |
| **Quality signal** | download count | download count + `docs.rs` quality |

> 💡 **Pro tip:** Every crate published to crates.io automatically gets
> beautiful documentation at `docs.rs/<crate-name>`.
> Visit https://docs.rs/tungstenite to see all the types and functions we'll use.

---

## 🔍 What `tungstenite` Gives Us

```
tungstenite
 │
 ├── accept(tcp_stream)              → performs the WebSocket handshake
 │                                     returns a WebSocket<TcpStream>
 │
 ├── WebSocket<TcpStream>
 │    ├── .read()                    → block until a Message arrives
 │    ├── .send(message)             → send a Message to the client
 │    └── .close(None)              → cleanly close the connection
 │
 └── Message  (an enum)
      ├── Message::Text(String)      → a UTF-8 text frame
      ├── Message::Binary(Vec<u8>)   → raw bytes frame
      ├── Message::Ping(Vec<u8>)     → keepalive ping
      ├── Message::Pong(Vec<u8>)     → response to ping
      └── Message::Close(...)        → close the connection
```

---

## 📄 Our `Cargo.toml` Right Now

```toml
[package]
name    = "websocket_echo_server"
version = "0.1.0"
edition = "2024"

[dependencies]
tungstenite = "0.26"   # WebSocket library — sync, no async needed
```

That's it! One dependency. Let's verify Cargo can download and compile it:

```bash
cargo build
# Cargo will:
# 1. Read Cargo.toml
# 2. Download tungstenite + all its dependencies from crates.io
# 3. Compile everything
# 4. Write Cargo.lock with exact versions
```

---

## ✅ Summary of Step 2

| Concept | Python | Rust |
| :--- | :--- | :--- |
| Registry | PyPI | crates.io |
| Package manager | pip | cargo |
| Dependency file | requirements.txt | Cargo.toml |
| Lock file | pip freeze output | Cargo.lock (auto) |
| Install | `pip install x` | `cargo add x` |
| Docs | varies | docs.rs (always) |
| Version spec | `>=1.0,<2.0` | `"1.0"` (semver) |

- ✅ `Cargo.toml` = what you **want** (your intent)
- ✅ `Cargo.lock` = what you **get** (exact, reproducible, cryptographically verified)
- ✅ `tungstenite = "0.26"` is now our WebSocket engine
- ✅ `cargo build` downloads + compiles everything automatically

---

## 🔭 What's Next?

**Step 3** — Writing the actual echo server code. We'll use `tungstenite::accept()`
and handle the `Message` enum to echo every message back to the client.
