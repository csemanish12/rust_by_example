# Simple Synchronous Rust Web Server
A deep-dive learning project transitioning from Python to Rust by building a raw TCP web server using the standard library.

---

## 1. Memory Management: The "Janitor" vs. "The Contract"
In Python, memory is managed by a **Garbage Collector (GC)**. In Rust, it is managed by **Ownership**.

### Comparison Table: Cleanup Strategies
| Feature | Python (GC) | Rust (Ownership/Drop) |
| :--- | :--- | :--- |
| **Cleanup Logic** | "I'll find unused objects eventually." | "I'll destroy this exactly when the scope ends." |
| **Responsibility** | The Runtime (Python VM) | The Compiler (Rustc) |
| **Impact** | Occasional pauses for cleanup (Stop-the-world). | Zero runtime overhead. |

### The "Drop" Mechanism (Deterministic Destruction)
In Rust, when a variable goes out of scope, the compiler inserts a `drop()` call automatically.

**Python Example (Non-deterministic):**
```python
def handle():
    f = open("log.txt")
    # File might stay open until GC runs later unless explicitly closed
```

**Rust Example (Deterministic):**
```rust
fn handle() {
    let f = File::open("log.txt");
} // <--- 'f' is CLOSED and memory freed exactly here.
```

---

## 2. Moving vs. Referencing
In Python, everything is a shared reference. In Rust, passing data **Moves** it by default.

**Python (Shared access):**
```python
def process(data):
    print(data)

my_list = [1, 2, 3]
process(my_list)
print(my_list) # Success! 'main' and 'process' share the list.
```

**Rust (Ownership Move):**
```rust
fn process(stream: TcpStream) {
    // Ownership is moved here
} // 'stream' is dropped here.

fn main() {
    let stream = ...;
    process(stream);
    // println!("{:?}", stream); // COMPILE ERROR: Value used after move.
}
```

---

## 3. The Borrow Checker & Mutability
Rust variables are **immutable** by default. To change something, you must explicitly declare it `mut`.

| Concept | Python | Rust |
| :--- | :--- | :--- |
| **Default state** | Mutable | Immutable |
| **References** | Implicit / Shared | Explicit (`&` for borrow) |
| **Concurrency** | Thread-unsafe (Race conditions) | Thread-safe (Compile-time checks) |

### The "Golden Rule" of Borrowing:
At any given time, you can have **EITHER**:
* One mutable reference (`&mut T`)
* **OR** Any number of immutable references (`&T`)
* **BUT** you can never have both at the same time.

---

## 4. Troubleshooting History (Hurdles Cleared)

| Error Encountered | Root Cause | Fix |
| :--- | :--- | :--- |
| `println(...)` | Used Python function syntax. | Use the macro: `println!(...)`. |
| `net::Path{A, B}` | Misplaced curly braces. | Use `net::{A, B}`. |
| `Result` mismatch | Forgot to "open the box." | Call `.unwrap()` to get the value. |
| `cannot borrow as mutable` | Variables are frozen by default. | Added `mut` to the parameter: `mut stream: TcpStream`. |

---

## How to Run
1. `cargo run`
2. Open `http://127.0.0.1:7878` in your browser.