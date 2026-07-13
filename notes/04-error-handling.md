# 04 — Error Handling

**Covers:** the `?` operator's actual mechanism (`From`-based conversion),
defining custom error types with `thiserror`, combining arbitrary error
types in a binary with `anyhow` (including `.context()` and error chains),
and why `.unwrap()`/`.expect()`/panics are avoided outside tests and startup
code.

---

## Why Rust needs this at all

Python normally handles failure with exceptions: a function throws, and
unless something catches it, it propagates upward automatically and
silently, invisible from the function's signature. Rust has no exceptions
for this. Failure is just a normal return value — `Result<T, E>` (an
ordinary enum, see topic 3) — and propagating it upward is something you
write explicitly.

## The `?` operator

```rust
fn parse_number(input: &str) -> Result<i32, std::num::ParseIntError> {
    input.parse::<i32>()
}

fn main() -> Result<(), std::num::ParseIntError> {
    let value = parse_number("42")?;
    println!("Parsed: {value}");

    Ok(())
}
```

- `input.parse::<i32>()` — a method every string has, trying to interpret
  the string as an `i32` and returning `Result<i32, ParseIntError>`.
  `ParseIntError` is the standard library's real (non-`String`) error type
  for "this string wasn't a valid number."
- `fn main() -> Result<(), E>` — `main` itself is allowed to return a
  `Result`. `()` is Rust's "empty"/unit type — `Result<(), E>` reads as
  "nothing useful on success, or an `E` on failure."
- `parse_number("42")?` — if the `Result` is `Err`, `?` immediately returns
  that `Err` from the *current* function, right there, skipping everything
  after it. If it's `Ok`, `?` unwraps it and gives you the inner value
  directly — execution continues normally with that value in hand.

When `main` itself returns an `Err`, Rust's runtime prints it (using
`Debug`, not `Display`) and exits the process with a **non-zero exit code**
— this is the actual mechanism a shell script or CI pipeline uses to detect
failure:

```
Error: ParseIntError { kind: InvalidDigit }
```

```bash
$ echo $?
1        # non-zero = failure
```

A successful run exits `0`. This distinction (`$?` after any command) is
exactly how failure is detected outside the program itself.

### `Display` vs `Debug`

- `{e}` in `println!`/`format!` calls the `Display` trait — a clean,
  human-readable message (`invalid digit found in string`).
- `{e:?}` calls `Debug` — a developer-facing dump exposing the value's
  actual internal shape (`ParseIntError { kind: InvalidDigit }`).

`Display` is for end users; `Debug` is for developers debugging or logging.
Any type printed with `{}` must implement `Display`; `{:?}` requires
`Debug`. Both can be derived automatically for your own types
(`#[derive(Debug)]`), and `thiserror` (below) derives `Display` for you too.

## The real mechanism behind `?`

`?` doesn't just "return early on `Err`" — it tries to **convert** the
error value into the function's declared error type first, using the
`From` trait. Proof: combining two functions with genuinely different
error types in one function fails to compile:

```rust
fn read_file_content(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

fn read_and_parse(path: &str) -> Result<i32, std::num::ParseIntError> {
    let content = read_file_content(path)?;   // io::Error, but fn returns ParseIntError
    let number = parse_number(content.trim())?;
    Ok(number)
}
```

```
error[E0277]: `?` couldn't convert the error to `ParseIntError`
  |
  | the trait `From<std::io::Error>` is not implemented for `ParseIntError`
  |
  = note: the question mark operation (`?`) implicitly performs a
    conversion on the error value using the `From` trait
```

Correctly rejected — there's no sensible conversion from "file couldn't be
read" to "string wasn't a number," and none should exist. But this means
any function combining two different fallible operations needs a single
error type both underlying errors can convert *into*. That's the actual
motivating problem `thiserror` solves.

---

## `thiserror` — typed errors for library crates

```toml
# Cargo.toml
[dependencies]
thiserror = "2"
```

```rust
#[derive(thiserror::Error, Debug)]
enum AppError {
    #[error("failed to parse number: {0}")]
    Parse(#[from] std::num::ParseIntError),

    #[error("failed to read file: {0}")]
    Io(#[from] std::io::Error),
}

fn read_and_parse(path: &str) -> Result<i32, AppError> {
    let content = read_file_content(path)?;      // io::Error -> AppError::Io, automatic
    let number = parse_number(content.trim())?;  // ParseIntError -> AppError::Parse, automatic
    Ok(number)
}
```

- `#[derive(thiserror::Error, Debug)]` — auto-implements the standard
  `Error` trait, `Display` (via each variant's `#[error("...")]` template),
  and `Debug`. Without `thiserror` this is all hand-written boilerplate.
- Each variant wraps the *original* error as its single field — no
  information from the underlying failure is lost, just given a home in
  your own type.
- `#[error("failed to parse number: {0}")]` — the `Display` message for
  that variant; `{0}` interpolates the wrapped field's own `Display`
  output, so the original message is embedded inside yours.
- `#[from]` on a field — generates a `From<OriginalError> for AppError`
  implementation automatically. This is exactly what `?` needs: now both
  `io::Error` and `ParseIntError` have a real conversion path into the
  single `AppError` type, no `.map_err(...)` needed anywhere.

Real output, layering both messages:

```
Error: failed to read file: No such file or directory (os error 2)
```
(`"failed to read file: "` is `AppError::Io`'s own template; `"No such
file or directory (os error 2)"` is the original `io::Error`'s `Display`
message, embedded via `{0}`.)

**House convention**: variant names are usually chosen to echo the
category/source of the error (`Parse` for `ParseIntError`, `Io` for
`io::Error`) — purely a readability convention, not a compiler rule. Any
valid identifier works.

---

## `anyhow` — combining arbitrary errors in a binary crate

```toml
# Cargo.toml
[dependencies]
anyhow = "1"
```

```rust
fn read_and_parse(path: &str) -> anyhow::Result<i32> {
    let content = read_file_content(path)?;
    let number = parse_number(content.trim())?;
    Ok(number)
}
```

No custom enum at all. `anyhow::Result<T>` is a type alias for
`Result<T, anyhow::Error>`, and `anyhow::Error` has a **blanket**
conversion built in that accepts *any* type implementing the standard
`Error` trait. So both `?`s — one producing `io::Error`, one producing
`ParseIntError` — convert automatically, with zero `#[from]`, zero enum
definition.

**The trade-off, precisely:** `thiserror` gives up nothing (callers can
still `match` on the specific error kind) at the cost of writing an enum.
`anyhow` gives up that ability (callers only ever see "an error happened,"
not which specific kind) in exchange for zero boilerplate. This is exactly
the right trade at the top of a binary, where there's usually no further
caller that needs to pattern-match the failure — hence the house rule:
**library crates use `thiserror`, binary crates use `anyhow`.**

### `.context()` — attaching human-readable explanation as errors propagate

```rust
use anyhow::Context; // .context() is a trait method — needs its trait in scope to be callable

fn read_and_parse(path: &str) -> anyhow::Result<i32> {
    let content = read_file_content(path)
        .context("failed to read input file")?;
    let number = parse_number(content.trim())?;
    Ok(number)
}
```

With `{e}` (`Display`), only the outermost context shows:

```
Error: failed to read input file
```

The original `io::Error` isn't lost — just hidden by default. With `{e:?}`
(`Debug`), `anyhow` prints the **full chain**:

```
Error: failed to read input file

Caused by:
    No such file or directory (os error 2)
```

This is the real-world pattern: attach `.context("...")` at each layer as
an error bubbles up through several function calls, so a single top-level
log line shows a readable trail of what failed at *each* level — not just
a raw OS error with no indication of which of your functions triggered it.

---

## `.unwrap()` / `.expect()` and panics — why they're avoided

```rust
fn main() {
    let content = std::fs::read_to_string("nonexistent.txt").unwrap();
    println!("{content}");
}
```

```
thread 'main' (538950) panicked at src/bin/04_panic.rs:2:62:
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "No such file or directory" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

```bash
$ echo $?
101
```

This is a **panic** — fundamentally different from a returned `Err`. It's
an abrupt, uncontrolled abort of the whole process: no `Result` is
returned, no calling code gets a chance to intervene, no `.context()` layer
is possible, just a raw crash message and exit code `101`. Compare to the
`anyhow` version above: same underlying failure, but there it became a
normal `Err`, propagated cleanly via `?`, got a human-readable `.context()`
attached, and `main` exited with a clean, controlled code (`1`).

**Why this matters:** a function deep in a call chain has no way to know
whether its caller could have handled the failure gracefully — retried,
logged and continued, shown a friendly message. `.unwrap()`/`.expect()`
unilaterally decide "crash the entire program right now" on behalf of
every caller above it, for any input that hits that path. That's why
they're avoided everywhere except:
- **Tests** — a failing test *should* panic; that's how a test fails.
- **Narrow startup code in `main`** — e.g. a required config file that
  truly can't be missing; crashing immediately at boot (before any request
  is in flight to disrupt) can be a legitimate, deliberate design choice.

Everywhere else, returning `Result` and letting `?`/`.context()` propagate
the failure keeps "should this be fatal?" a decision made by whoever is
actually equipped to make it — usually much higher up the call stack, or
never, if the failure is recoverable.

---

## Recap

- Rust has no exceptions. Failure is a normal `Result<T, E>` return value;
  propagating it is explicit.
- `?` returns early on `Err`, converting the error into the current
  function's declared error type via the `From` trait — this conversion
  is *why* combining two different error-returning calls under one return
  type requires either a shared error type or a blanket converter.
- `thiserror` (library crates): define a typed error enum, `#[from]` on
  each wrapped field auto-generates the `From` impl `?` needs, `#[error(...)]`
  derives `Display`. Callers can still `match` on the specific failure.
- `anyhow` (binary crates): `anyhow::Result<T>` accepts any `Error`-
  implementing type via a blanket conversion — zero enum needed.
  `.context("...")` (needs `use anyhow::Context;`) layers a human-readable
  explanation onto an error as it propagates; `{:?}` reveals the full
  "Caused by" chain, `{}` shows only the top layer.
- `.unwrap()`/`.expect()` panic on `Err`/`None` — an uncontrolled process
  abort, not a normal `Result`. Reserved for tests and narrow startup-only
  code; everywhere else, propagate via `Result`/`?` instead.
