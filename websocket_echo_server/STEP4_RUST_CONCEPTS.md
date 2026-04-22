# Step 4 — New Rust Concepts Deep Dive

> *"Everything in `main.rs` that Python hides from you — explained."*

---

## The Four Concepts We'll Cover

```
1. enum   — a type that can be one of many variants (carrying data)
2. match  — exhaustive pattern matching (Rust's super-powered switch/isinstance)
3. Result — the Rust way to handle errors instead of try/except
4. loop   — Rust's infinite loop and how to break out of it
```

---

## 1. `enum` — A Type That Is One of Many Things

### 🐍 Python's Way

Python doesn't have real enums for this pattern. You'd check the **runtime type**:

```python
message = websocket.recv()

if isinstance(message, str):      # could be Text
    handle_text(message)
elif isinstance(message, bytes):  # could be Binary
    handle_binary(message)
# ❌ Nothing stops you from forgetting a case — Python won't warn you
```

Or you might use Python's `enum.Enum` — but it can't carry data per-variant:

```python
from enum import Enum

class Color(Enum):
    RED   = 1
    GREEN = 2
    BLUE  = 3
# ❌ Each variant is just a value — can't attach different data to each
```

### 🦀 Rust's Way

Rust `enum` variants can each carry **different types of data**:

```rust
// tungstenite defines this internally — simplified view:
enum Message {
    Text(String),          // variant carrying a String
    Binary(Vec<u8>),       // variant carrying a byte vector
    Ping(Vec<u8>),         // variant carrying ping payload
    Pong(Vec<u8>),         // variant carrying pong payload
    Close(Option<CloseFrame>), // variant carrying optional close info
}
```

```
Message
 │
 ├── Text    ──► contains a String       ("Hello!")
 ├── Binary  ──► contains Vec<u8>        ([0x89, 0x50, ...])
 ├── Ping    ──► contains Vec<u8>        (ping payload)
 ├── Pong    ──► contains Vec<u8>        (pong payload)
 └── Close   ──► contains Option<...>   (close reason, or None)
```

A `Message` value is **exactly one** of these at any given time. The compiler
knows this. You can't accidentally treat a `Binary` as a `Text`.

### Creating an enum value

```rust
// Creating variants
let msg1 = Message::Text(String::from("Hello!"));
let msg2 = Message::Binary(vec![1, 2, 3]);
let msg3 = Message::Close(None);

// Python equivalent (rough)
msg1 = "Hello!"          # just a str — no type safety
msg2 = b"\x01\x02\x03"  # just bytes  — no type safety
```

### Real-world analogy

Think of `Message` like a **sealed envelope** that can contain one of several things:
```
📨 Text envelope    → open it → you get a String
📦 Binary envelope  → open it → you get bytes
🔌 Close envelope   → open it → you get a reason (or nothing)
```
`match` is how you **open the envelope** and handle what's inside.

---

## 2. `match` — Exhaustive Pattern Matching

### 🐍 Python's Closest Thing

```python
# Python if/elif chain — not exhaustive, compiler won't warn if you miss a case
if isinstance(message, str):
    print(f"Text: {message}")
elif isinstance(message, bytes):
    print(f"Binary: {len(message)} bytes")
# ← forgot Close? Python doesn't care. Bug silently introduced.

# Python 3.10+ structural pattern matching — closer, but still not exhaustive
match message:
    case str(text):
        print(f"Text: {text}")
    case bytes(data):
        print(f"Binary: {len(data)} bytes")
    # still no compiler warning if you forget a case
```

### 🦀 Rust's `match` — The Compiler Enforces Completeness

```rust
match message {
    Message::Text(ref text)    => { println!("Text: {text}"); }
    Message::Binary(ref bytes) => { println!("Binary: {} bytes", bytes.len()); }
    Message::Ping(_)           => { /* auto-handled */ }
    Message::Close(_)          => { break; }
    _                          => { /* catch-all */ }
}
// ✅ If you remove '_' and forget a variant → COMPILE ERROR
// The compiler guarantees you handle every possible case
```

### The `_` Wildcard — The Catch-All

```rust
_ => {}   // "anything else I didn't explicitly name → do nothing"
```

```python
else:     # Python's equivalent
    pass
```

### Destructuring — Extracting Data from Variants

```rust
Message::Text(ref text) => {
//            ────────
//            'text' is now a &String — the data inside the Text variant
//            you "open the envelope" and name what's inside
    println!("Got: {text}");
}
```

```python
# Python — you already have the value, no destructuring needed
if isinstance(message, str):
    text = message   # just an assignment
    print(f"Got: {text}")
```

### `ref` — Borrow Instead of Move

```rust
Message::Text(ref text) => {
//            ───
// WITHOUT ref: 'text' would MOVE the String out of 'message'
//              → message is partially moved → can't call message.clone()
//
// WITH ref:    'text' is a BORROW of the String inside message
//              → message still fully owns the String → can clone it
    websocket.send(message.clone()).unwrap(); // ← works because message is intact
}
```

```python
# Python — you never move data, everything is a reference, no issue
text = message   # both 'text' and 'message' reference the same object
send(message)    # message still usable — Python GC handles it
```

> **Mental model:** `ref` is like saying "let me look inside the envelope without
> taking anything out". Without `ref` you'd be pulling the contents out and now
> the envelope is empty.

### `match` as an Expression — Returns a Value

Unlike Python's `if/elif`, Rust's `match` is an **expression** — it produces a value:

```rust
// match returns a value — assigned directly
let status = match score {
    90..=100 => "A",
    80..=89  => "B",
    70..=79  => "C",
    _        => "F",
};
println!("Grade: {status}"); // "A", "B", etc.
```

```python
# Python equivalent — ternary or if/elif block
status = "A" if score >= 90 else "B" if score >= 80 else "F"
```

---

## 3. `Result<T, E>` — Rust's Way of Handling Errors

### 🐍 Python's Way — Exceptions

```python
try:
    message = websocket.recv()   # might raise an exception
    print(message)
except ConnectionClosed as e:
    print(f"Disconnected: {e}")
    break
except Exception as e:
    print(f"Error: {e}")
    break
```

Problems with exceptions:
- **Invisible** — a function's signature doesn't tell you it can fail
- **Unchecked** — you can forget the `try/except` and Python won't warn you
- **Surprising** — any function anywhere can raise any exception

### 🦀 Rust's Way — `Result<T, E>`

In Rust, functions that can fail return a `Result`:

```rust
// Result is just another enum:
enum Result<T, E> {
    Ok(T),    // success — contains the value of type T
    Err(E),   // failure — contains the error of type E
}
```

```rust
// websocket.read() returns Result<Message, Error>
// You CANNOT use the Message without handling the Err case
// The compiler will warn you if you ignore the Result

let result: Result<Message, Error> = websocket.read();
// result is either Ok(Message) or Err(Error) — must be handled
```

### Three Ways to Handle a `Result`

#### Way 1 — `.unwrap()` — "I'm sure it won't fail" (crashes on Err)
```rust
let message = websocket.read().unwrap();
// ✅ Simple, but panics the thread if there's an error
// Use for: prototyping, cases that truly cannot fail
```
```python
# Python equivalent — no try/except, just let it crash
message = websocket.recv()
```

#### Way 2 — `match` — Handle both cases explicitly (what we use)
```rust
let message = match websocket.read() {
    Ok(msg) => msg,             // success → extract and use the Message
    Err(e)  => {
        println!("Error: {e}"); // failure → log and break
        break;
    }
};
// ✅ Explicit, safe, clear — the right approach for a server
```
```python
# Python equivalent
try:
    message = websocket.recv()
except Exception as e:
    print(f"Error: {e}")
    break
```

#### Way 3 — `?` operator — Propagate the error up (for functions that return Result)
```rust
fn read_message(ws: &mut WebSocket<TcpStream>) -> Result<Message, Error> {
    let msg = ws.read()?; // if Err → return Err immediately (like 'raise' in Python)
    Ok(msg)
}
```
```python
# Python equivalent — just don't catch the exception, let it propagate
def read_message(ws):
    return ws.recv()  # exception propagates naturally
```

### `Result` vs Exceptions — Comparison

| | Python Exceptions | Rust `Result` |
| :--- | :--- | :--- |
| **Visible in signature?** | ❌ No | ✅ Yes — `fn foo() -> Result<T, E>` |
| **Forgettable?** | ❌ Yes — forget try/except | ⚠️ Warning if you ignore Result |
| **Forced to handle?** | ❌ No | ✅ Compiler warns you |
| **Performance** | Slow (stack unwinding) | Zero cost (just an enum) |

---

## 4. `loop {}` — Rust's Infinite Loop

### 🐍 Python's Way

```python
while True:
    message = websocket.recv()
    if should_stop:
        break
```

### 🦀 Rust's Way

```rust
loop {
    let message = match websocket.read() {
        Ok(msg) => msg,
        Err(_)  => break,  // ← exit the loop
    };

    match message {
        Message::Close(_) => break, // ← exit the loop
        _                 => { /* keep going */ }
    }
}
```

`loop` is exactly `while True` — it runs forever until `break` is called.

### Why `loop` instead of `while true`?

Rust has `while` too, but `loop` is preferred when:

```rust
// loop — the compiler KNOWS this runs forever
// It can return a value via 'break'
let result = loop {
    let val = compute();
    if val > 10 {
        break val;  // ← return 'val' from the loop expression
    }
};
```

```python
# Python — while True can also "return" via assignment
result = None
while True:
    val = compute()
    if val > 10:
        result = val
        break
```

In our echo server `loop` means:
> "Keep reading and echoing messages until the client disconnects or sends a Close frame."

---

## 🗺️ Everything Together — `main.rs` Annotated

```rust
loop {                                // ← infinite loop, like while True

    let message = match websocket.read() {   // ← Result handling
        Ok(msg) => msg,              //   Ok  → got a Message, continue
        Err(e)  => { break; }        //   Err → client gone, exit loop
    };
                                     //        ↑ Result<T,E>
    match message {                  // ← enum pattern matching
        Message::Text(ref text) => { //   destructure the Text variant
            websocket.send(message.clone()).unwrap();  // ref kept message intact
        }
        Message::Close(_) => { break; } // ← exit loop on Close
        _ => {}                          // ← catch-all, ignore others
    }
}
```

---

## 🧠 The Big Picture — What Rust Buys You

| Concept | Python | Rust |
| :--- | :--- | :--- |
| Tagged union types | `isinstance()` at runtime | `enum` — checked at compile time |
| Exhaustive case handling | ❌ Easy to forget a case | ✅ Compiler error if you miss one |
| Error handling | `try/except` — invisible, forgettable | `Result` — visible in types, enforced |
| Infinite loop | `while True` | `loop {}` — can return values |
| Borrowing inside match | N/A (everything is a ref) | `ref` — explicit borrow |

All of this means: **if it compiles, it almost certainly works correctly.**
The compiler has already checked the cases you'd normally find through runtime testing.

---

## ✅ Summary of Step 4

- ✅ **`enum`** — a type that is exactly one of several variants, each carrying its own data
- ✅ **`match`** — exhaustive pattern matching; compiler enforces you handle every variant
- ✅ **`ref`** — borrow data out of a match arm without moving it (keeps the original intact)
- ✅ **`Result<T,E>`** — functions that can fail return this instead of throwing exceptions
- ✅ **`match` on `Result`** — `Ok(val) => use val`, `Err(e) => handle error`
- ✅ **`loop {}`** — infinite loop (`while True`), exited via `break`

---

## 🔭 What's Next?

**Step 5** — Make the server handle **multiple clients at once** by wiring in our
`ThreadPool` from the multithreaded server project. Right now one slow client
blocks everyone else — sound familiar? 😄
