# 05b — Tokio & `#[tokio::main]`: just enough to read it

**Covers:** why `async fn` needs a runtime at all, what `#[tokio::main]`
actually expands to, and what the `tokio` feature flags you added mean.
This is a preview — full async/await (`tokio::spawn`, `timeout`,
`spawn_blocking`, channels) is its own later topic; this note only exists
so `05_async_trait.rs`'s boilerplate isn't a mystery.

---

## Why `async fn` alone doesn't run anything

`async fn foo() -> T` doesn't execute when you call it. Calling it just
constructs a value — a `Future<Output = T>` — that describes the work but
does nothing until something actively drives it forward. This is why
Rust's futures are called **lazy**: `foo()` on its own is inert, a plan of
what to do, not the doing of it.

`.await` is what advances a future. But `.await` itself can only be used
*inside another async function* — so ultimately, something outside all
async code has to kick off the first future and keep polling it until it's
done. That "something" is called an **executor** (or, more broadly, a
**runtime**, since it usually also handles things like timers and network
I/O, not just polling).

Rust's standard library deliberately ships **no** async runtime — it only
provides the `Future` trait and `async`/`.await` syntax. You must bring
your own executor. `tokio` is the de facto standard one in the ecosystem
(the other common one is `async-std`, less used now).

## What `#[tokio::main]` actually does

```rust
#[tokio::main]
async fn main() {
    let fetcher = Fetcher {};
    println!("{}", fetcher.run().await);
}
```

`fn main()` is not allowed to be `async` on its own — the OS calls `main`
directly, expecting an ordinary synchronous function, and there's no
runtime yet to poll a future even if it could return one. `#[tokio::main]`
is a macro that rewrites your `async fn main() { ... }` into roughly:

```rust
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let fetcher = Fetcher {};
            println!("{}", fetcher.run().await);
        })
}
```

- `Builder::new_multi_thread()` — constructs a runtime backed by a small
  pool of OS threads that cooperatively run many async tasks.
- `.enable_all()` — turns on tokio's I/O and timer drivers (needed the
  moment you touch a socket, file, or `tokio::time::sleep`; harmless to
  enable even if your code doesn't need them yet).
- `.block_on(async { ... })` — takes your original async body, wrapped as
  one future, and drives it to completion right there, **blocking** the
  real synchronous `main` thread until it finishes. This is the actual
  bridge between "ordinary synchronous program entry point" and "async
  code with `.await` in it."

So the macro's whole job is: build a runtime, then run your async `main`
body on it, synchronously from the OS's point of view. Everywhere you see
`#[tokio::main]`, mentally substitute "this creates a runtime and blocks on
the async body below" — nothing more mysterious than that.

## What `.await` did in your exercise

```rust
println!("{}", fetcher.run().await);
```

`fetcher.run()` returns a future (or, since `run` is behind
`#[async_trait::async_trait]`, a boxed future — see
[05-traits-and-trait-objects.md](05-traits-and-trait-objects.md)).
`.await` hands that future to the runtime and suspends the current task
until it resolves, at which point `.await` evaluates to the future's
output — here, the `String`. Your exercise awaited each call one after
another, so nothing ran concurrently; concurrency (`tokio::spawn`, running
several futures at once) is the actual point of the later async/tokio
topic, not this one.

## What the two feature flags you added mean

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- `"macros"` — turns on `#[tokio::main]` and `#[tokio::test]`. Without
  it, you'd have to construct the `Builder`/`block_on` call yourself by
  hand, as shown above.
- `"rt-multi-thread"` — turns on the multi-threaded scheduler that
  `Builder::new_multi_thread()` needs. (The alternative is a
  single-threaded scheduler, `"rt"` + `Builder::new_current_thread()`,
  used via `#[tokio::main(flavor = "current_thread")]` — fine for small
  programs, lower overhead, but only one OS thread ever runs your tasks.)

Tokio's feature flags are deliberately opt-in and granular (there's also
`"net"`, `"fs"`, `"time"`, `"sync"`, `"io-util"`, ...) so a binary only
pays compile time and binary size for the pieces it actually uses. `"full"`
turns on everything at once — fine for exercises/learning, usually
trimmed down in real projects.
