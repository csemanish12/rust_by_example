# 01 — Cargo & Project Structure

**Covers:** package manifest anatomy, binary vs. library crates, `pub` visibility,
function syntax basics, macros, the compile-then-run workflow, `Cargo.lock` vs
`Cargo.toml`, splitting code across files with `mod`, the `use` statement, and
Rust editions.

Written while coming from a Python/Flask background — comparisons below are to
help the concepts stick, not because Rust and Python work the same way.

---

## `Cargo.toml` — the package manifest

Every crate (Rust's word for "package") has one. Rough Python equivalent:
`pyproject.toml` + a lockfile, combined.

```toml
[package]
name = "rust_exercises"
version = "0.1.0"
edition = "2024"

[dependencies]
```

- `[package]` — metadata about this crate: `name`, `version` (semver, like a
  PyPI package version), `edition` (see the Editions section below).
- `[dependencies]` — third-party crates this one depends on, same role as
  `requirements.txt`. A **local** dependency (another crate on disk, not from
  the registry) looks like:

  ```toml
  [dependencies]
  mylib = { path = "../mylib" }
  ```

## Binary crate vs. library crate

`cargo new my_app` creates a **binary crate**: a `src/main.rs` with a `main()`
function — something you *run*. It produces an executable.

`cargo new --lib my_lib` creates a **library crate**: a `src/lib.rs` with *no*
`main()` — something other code *depends on and calls into*. Closer to a
Python package you `import`, not a script you execute.

The file name is what Cargo uses to decide which kind you get — `src/main.rs`
and `src/lib.rs` are special names Cargo looks for automatically, no config
needed:

- `src/main.rs` → binary crate
- `src/lib.rs` → library crate
- `src/bin/*.rs` → each file here becomes an *additional* separate binary in
  the same crate

You *can* override these default paths via `[[bin]] path = "..."` or
`[lib] path = "..."` in `Cargo.toml`, but it's rare in practice.

### Why can't you "run" a library crate?

A library's functions clearly execute — so what makes it different from a
binary? The difference is **what artifact gets produced**, not whether the
code inside it can run:

- **Binary crate** compiles down to a standalone OS executable (a real
  `.exe`/Mach-O file under `target/debug/`). The OS can launch it directly as
  its own process — that's what `Running 'target/debug/rust_exercises'`
  meant earlier.
- **Library crate** compiles down to a `.rlib` file (Rust's static library
  format). This is **not launchable by the OS** — there's no process to
  start, because there's no `main()`, i.e. no defined entry point for the OS
  to jump to.

So calling `mylib::add(2, 3)` from `main.rs` isn't "running the library" as
its own thing — it's linking `mylib`'s compiled code *into* the
`rust_exercises` binary, and calling that function *from within* the
`rust_exercises` process. Library code only ever executes as part of some
binary's process; it never gets a process of its own.

Proof: run `cargo run` from inside a library crate directory (e.g. `mylib/`)
and Cargo refuses:

```
error: a bin target must be available for `cargo run`
```

There's nothing to run — no `main()` was ever defined, so no executable was
even produced.

Python analogy: a library crate is like a `.py` module you `import` and call
functions from — it never runs standalone, only inside whatever script
imported it. A binary crate is the `.py` file you'd actually invoke with
`python script.py`.

## A minimal binary

```rust
fn main() {
    println!("Hello, world!");
}
```

- `fn main()` — every binary crate needs exactly one `main` function; program
  execution starts here. Mandatory, unlike Python's optional
  `if __name__ == "__main__":` guard.
- `println!` — note the `!`. This is not a function call, it's a **macro**
  (macros always end in `!`). Rough equivalent of Python's `print(...)`. No
  need to understand how macros work yet — just recognize the `!` and read on.
- `{ }` braces define scope/blocks. Indentation is just style here, not
  syntax — unlike Python, where indentation *is* the block structure.
- Statements end in `;`. Rust requires this; Python doesn't.

## A minimal library

```rust
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}
```

- `pub` — marks the function **public**: visible to code *outside* this
  crate. Without `pub`, an item is private to the crate by default — the
  opposite default from Python, where everything is importable unless you
  underscore-prefix it by convention.
- `left: u64, right: u64` — parameters with explicit types (`u64` = unsigned
  64-bit integer). Rust always requires type annotations on function
  parameters and return values — no inference here, unlike Python's optional
  type hints.
- `-> u64` — return type.
- `left + right` with **no** `return` keyword and **no** trailing `;` — in
  Rust, the last expression in a function body is automatically the return
  value, as long as it has no trailing semicolon. This is idiomatic Rust; you
  see it constantly instead of explicit `return`.

## Calling one crate from another

```rust
// main.rs, with `mylib` added as a path dependency in Cargo.toml
fn main() {
    let result = mylib::add(2, 3);
    println!("2 + 3 = {result}");
}
```

`mylib::add(...)` — `::` is how you reach into another crate or module's
namespace to get an item out of it. Roughly like Python's `mylib.add(2, 3)`,
but Rust reserves `.` for method calls on values and uses `::` for
namespacing (crate → module → item).

## Is there an "import" statement?

Yes — it's `use`. The example above didn't need it because it used the fully
qualified path (`mylib::add`). `use` just shortens repeated references:

```rust
use mylib::add;

fn main() {
    let result = add(2, 3); // no more mylib:: prefix needed
}
```

Roughly: `use mylib::add;` ≈ Python's `from mylib import add`.

Important distinction: adding a crate under `[dependencies]` in `Cargo.toml`
only makes it *available to link against*. It does **not** inject any names
into scope by itself — you still need either the full `crate_name::path` or a
`use` statement to actually reference something from it.

## Splitting code across multiple files

A crate is not limited to one `main.rs`/`lib.rs` file. Use `mod` to declare a
submodule living in its own file:

```rust
// lib.rs
mod keystore;              // looks for keystore.rs (or keystore/mod.rs) next to this file
pub use keystore::KeyStore; // re-export so callers can write mylib::KeyStore
                             // instead of mylib::keystore::KeyStore
```

Each `mod foo;` line tells Rust to find `foo.rs` in the same directory and
treat it as a submodule of the current file. This is how real projects avoid
one giant `lib.rs` — conceptually close to a Python package's `__init__.py`
pulling in sibling submodule files.

## Compiling and running — `cargo run`

```
$ cargo run
   Compiling rust_exercises v0.1.0 (/path/to/rust_exercises)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.59s
     Running `target/debug/rust_exercises`
Hello, world!
```

- **Rust is compiled, not interpreted.** Unlike `python app.py`, which runs
  source directly, `cargo run` first turns `.rs` source into machine code
  (the "Compiling" step) before anything executes.
- `Finished 'dev' profile [unoptimized + debuginfo] ...` — a debug build
  finished: compiles fast, runs slower than a release build would.
- `Running 'target/debug/...'` — cargo then executes the compiled binary it
  just produced.
- The last line is the program's actual output.

`cargo run` = compile + execute in one command. `cargo build` alone only
compiles, leaving the binary sitting in `target/debug/` without running it.

When a dependency is involved, you'll also see it resolved and compiled
first, since dependencies must build before the crate that uses them:

```
    Locking 1 package to latest Rust ... compatible version
      Adding mylib v0.1.0 (...)
   Compiling mylib v0.1.0 (...)
   Compiling rust_exercises v0.1.0 (...)
```

## `Cargo.toml` vs `Cargo.lock`

Two different files, easy to mix up:

- **`Cargo.toml`** — hand-written. Uses `[package]` (single brackets = one
  table, describing *this* crate) and `[dependencies]`.
- **`Cargo.lock`** — auto-generated and rewritten by Cargo after every build.
  Never hand-edit it. It pins exact resolved versions of every dependency in
  the graph, so builds are reproducible — the same role as a
  `pip freeze`/`poetry.lock` output.

`Cargo.lock` uses `[[package]]` — **double** brackets. In TOML, double
brackets mean "array of tables," i.e. a *list*, where each `[[package]]` line
starts a new entry in that list. With more than one dependency you get
multiple stacked `[[package]]` blocks, each with its own `name`, `version`,
`source`, `checksum`, `dependencies` fields:

```
[[package]]
name = "mylib"
version = "0.2.0"

[[package]]
name = "rust_exercises"
version = "0.1.0"
dependencies = [
 "mylib",
]
```

## Workspaces — tying multiple crates together

A **workspace** is a root `Cargo.toml` that lists multiple crates as members,
built and lockfile-managed together as a unit:

```toml
[workspace]
resolver = "3"
members = ["mylib", "rust_exercises"]
```

- `members` — the list of crate directories included in this workspace.
- `resolver` — which version of Cargo's dependency-resolution algorithm to
  use ("1", "2", or "3"). Newer editions expect newer resolver versions;
  Cargo will warn if there's a mismatch. Not worth digging into deeply — just
  know the line needs to be there and Cargo tells you if it's wrong.

Building from the workspace root (`cargo build`) compiles every member crate
in one shot, resolving all their dependencies together into a single
`Cargo.lock` at the workspace root instead of one per crate.

## Editions

`edition = "2021"` / `"2024"` in `[package]` selects which version of the
Rust language rules a crate compiles against. Editions are Rust's mechanism
for introducing breaking language changes without breaking old code — each
crate declares its own edition independently, and the compiler bridges
between crates using different editions within the same build.

For a new project, defaulting to the latest stable edition (currently 2024)
is a reasonable choice. Differences between recent editions are minor for a
beginner — not worth treating as a major decision.

---

## Recap

- Binary crate = `main.rs` + `main()`, produces a runnable executable.
  Library crate = `lib.rs`, no `main()`, meant to be depended on.
- `pub` controls visibility outside the crate.
- `::` reaches into another crate/module's namespace; `use` shortens
  repeated `::` paths (closer to Python's `import`).
- `mod foo;` pulls in `foo.rs` as a submodule — how real crates stay split
  across many files instead of one giant one.
- `Cargo.lock` is auto-generated, pins exact versions, and is structured as
  a TOML array of tables (`[[package]]`) — one block per resolved package.
- `[workspace]` + `members` builds and locks multiple crates together as one
  unit.
