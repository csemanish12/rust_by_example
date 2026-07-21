# 05 — Traits & Trait Objects

**Covers:** defining a `trait` and `impl Trait for Type`, default methods,
generic trait bounds (static dispatch), trait objects (`dyn Trait`,
`Box<dyn Trait>`, dynamic dispatch), and why plain traits can't have
`async fn` (the `async_trait` macro).

---

## What a trait actually is

A trait is a **contract**: it says "any type that implements me guarantees
it has these methods, with these signatures." A trait holds no data itself
— no fields, just method signatures (and optionally default bodies).

Compare to what you already know:
- Python `Protocol`/ABC — a structural or explicit contract, enforced
  loosely (or by a separate type checker, not the runtime itself).
- Rust `trait` — checked entirely at **compile time**. If a type claims
  `impl Greeter for Person`, the compiler verifies every required method
  exists with the exact right signature before the program ever runs.

Key difference from Python's duck typing: in Python, any object with a
`.greet()` method works, no declaration needed. Rust requires the explicit
`impl Trait for Type` block — even if `Person` already happens to have a
method named `greet`, it doesn't count as implementing `Greeter` unless
that `impl` block exists. This is deliberate: the compiler needs to know
statically which trait a method belongs to, since a type can implement
many traits that share method names.

## Anatomy

```rust
trait Greeter {
    fn greet(&self) -> String;   // signature only, no body -> "required method"
}

struct Person {
    name: String,
}

impl Greeter for Person {
    fn greet(&self) -> String {
        format!("Hello, my name is {}", self.name)
    }
}
```

- The `trait` block declares *what must exist*.
- The `impl Trait for Type` block *provides it*, for that specific type.
- `&self` — borrows the receiver immutably (same borrowing rules as
  topic 2). A trait method could instead take `&mut self` (needs exclusive
  access) or `self` (consumes/takes ownership) — the trait author picks
  based on what the method needs to do.

### No semicolon on the tail expression

```rust
fn greet(&self) -> String {
    format!("Hello, my name is {}", self.name)   // no `;` — this is the block's value
}
```

A block's last expression, if it has no trailing semicolon, becomes the
block's value — and here, the function's return value. Add a semicolon and
the expression becomes a statement, which evaluates to `()` (unit); the
block's value becomes `()`, which doesn't match the declared `-> String`
return type, and the compiler rejects it (`E0308: mismatched types`). This
rule isn't trait-specific — it applies to any block (`fn` bodies, `if`
branches, `match` arms, loop bodies).

## Why traits earn their keep

With a single struct, a trait looks like unnecessary ceremony — you could
just write `impl Person { fn greet(&self) -> String {...} }` directly, no
trait involved. Traits pay off once **multiple types need to be treated
uniformly**:

- A function can accept "anything implementing `Greeter`" instead of one
  specific concrete type — a generic bound, `fn f<T: Greeter>(x: T)`.
- A collection can hold *different* concrete types together, as long as
  they all implement the same trait — a trait object, `dyn Greeter`.

This is exactly the shape of a `Module` trait you'll often see in larger
Rust codebases: `start`/`stop`/`name` declared once as a trait, and many
different modules each `impl Module for TheirStruct`, so calling code can
hold a `Vec<Box<dyn Module>>` and call `.start()` on all of them without
caring which concrete type each one is.

## Default methods

A trait method can carry a body directly in the trait definition. Any
`impl` that doesn't override it just inherits that default — the
implementing type only has to supply the *other*, required methods.

```rust
trait Greeter {
    fn name(&self) -> String;                // required — no default body

    fn greet(&self) -> String {               // has a default body
        format!("Hello, my name is {}", self.name())
    }
}
```

A type implementing `Greeter` must define `name()`, but gets `greet()` for
free unless it explicitly overrides it with its own `fn greet(&self) ->
String { ... }` in its `impl` block.

## Default vs override, and field/method name shadowing

```rust
struct Robot { id: u32 }

impl Greeter for Robot {
    fn name(&self) -> String {
        format!("Unit-{}", self.id)
    }

    fn greet(&self) -> String {                      // overrides the trait default
        format!("BEEP BOOP I AM {}", self.name())
    }
}
```

If a type's `impl` block defines a method the trait already gives a
default for, the type's own version wins — the trait default is simply
never consulted for that type. Types that don't override it fall through
to the trait's default body.

Also worth noticing: a struct field and a trait method can share the same
identifier (e.g. field `self.name` vs method `self.name()`) with zero
ambiguity — Rust keeps field access and method calls in separate
namespaces, and the `()` always picks the method. This is stricter than
Python, where `self.name` could be a plain attribute or a bound method
depending on what was assigned to it.

## Static dispatch (so far)

Every call so far (`person.greet()`, `robot.greet()`) is resolved entirely
at **compile time** — the compiler knows the concrete type of each
variable, so it just calls the right function directly, as if traits
weren't even involved at runtime. This is **static dispatch**, and it's
the default, zero-cost case in Rust. It only breaks down when you need a
single piece of code to work over a type that isn't known until runtime
(e.g. a mixed collection of different `Greeter`-implementing types) — that
case needs dynamic dispatch, covered below.

## Generic bound (still static dispatch)

```rust
fn announce<T: Greeter>(g: &T) {
    println!("Announcing: {}", g.greet());
}
```

`<T: Greeter>` is a **trait bound**: "whatever concrete type `T` is, it
must implement `Greeter`." Without the bound, `g.greet()` wouldn't compile
— the compiler has no idea a bare generic `T` has a `.greet()` method
unless a bound says so.

This is still fully static. Rust doesn't generate one function that
branches on type at runtime — it **monomorphizes**: it generates a
separate concrete copy of `announce` for every distinct type it's actually
called with (conceptually `announce_person`, `announce_robot`). Each copy
is ordinary direct-call code, zero runtime overhead — the cost is paid at
compile time (larger binary, more codegen), not at runtime.

## Trait objects — `dyn Greeter`, `Box<dyn Greeter>`

Monomorphization requires knowing every concrete type *at compile time*.
That breaks down the moment you need one collection or one function
parameter to hold/accept genuinely different concrete types chosen at
**runtime** — e.g. a `Vec` mixing `Person` and `Robot` together.

### Why `Vec<T>` alone can't mix types

A `Vec<T>` is one contiguous, growable buffer — every element must be the
same fixed size, so the compiler can compute offsets. `Person` and `Robot`
have different sizes (a `String` vs a `u32`), so there's no single
concrete `T` both fit. `dyn Greeter` alone is worse: it names no single
type at all, so it has **no fixed size** (Rust calls this *unsized*,
`?Sized`) — it can't sit directly in a `Vec` slot or on the stack.

### `Box` makes it fit

`Box<T>` is a single heap allocation holding one `T`; the `Box` handle
itself is a fixed-size pointer, regardless of what it points to. So
`Box<Person>` and `Box<Robot>` are the same size as *handles*, even though
`Person` and `Robot` aren't. `Vec<Box<dyn Greeter>>` works because the
`Vec` only ever stores same-size `Box` pointers contiguously — each `Box`
points off to a differently-typed, differently-sized value on the heap.

Concretely, `Box<dyn Greeter>` is a **fat pointer** — two words:
1. a pointer to the concrete value's data on the heap
2. a pointer to a **vtable** — a small table of function pointers for that
   concrete type's trait methods, generated by the compiler per `impl`

```rust
fn announce_dyn(g: &dyn Greeter) {
    println!("{}", g.greet());
}

let mut greeters: Vec<Box<dyn Greeter>> = Vec::new();
greeters.push(Box::new(person));   // Person moved onto the heap, Box moved into the Vec
greeters.push(Box::new(robot));

for item in greeters.iter() {      // .iter() borrows the Vec; item: &Box<dyn Greeter>
    announce_dyn(item.as_ref());   // unwraps one layer: &Box<dyn Greeter> -> &dyn Greeter
}
```

`g.greet()` on a `&dyn Greeter` can't be resolved at compile time — the
compiler emits code that follows the vtable pointer **at runtime** and
jumps to whichever concrete `greet` is sitting in that slot. This is
**dynamic dispatch**: one extra pointer-chase per call, traded for not
needing to know the concrete type until runtime. Static dispatch
(exercise 3) has zero call overhead but needs every type known up front;
dynamic dispatch (`dyn`) costs a vtable lookup but allows genuinely mixed,
runtime-chosen types — pick based on which constraint you actually have.

### Ownership through the chain

- `Box::new(person)` **moves** `person` onto the heap; the `Box` now owns
  it. `.push(...)` then moves the `Box` into the `Vec`. After this, the
  original `person`/`robot` local variables are no longer usable (moved).
- `.iter()` **borrows** the `Vec` rather than consuming it — each `item`
  is `&Box<dyn Greeter>`, a reference, so the `Vec` (and its contents)
  is still intact after the loop.
- `.as_ref()` unwraps one layer of reference (`&Box<dyn Greeter> -> &dyn
  Greeter`) without taking ownership of anything — equivalent to the more
  manual `&**item` (deref the `&`, deref the `Box`, re-borrow), just named
  instead of symbolic.
- Nothing downstream of the initial `Box::new` calls ever takes ownership
  again — `announce_dyn(g: &dyn Greeter)` only ever borrows.

## Why plain traits can't have `async fn` in `dyn` position

`async fn foo() -> T` desugars to `fn foo() -> impl Future<Output = T>` —
each `async fn` body produces its own unique, compiler-generated anonymous
future type, sized according to whatever that specific body captures.
Plain `async fn` directly inside a trait now compiles fine for the
*static-dispatch* case (a generic bound, `fn f<T: Worker>`), because
monomorphization generates one concrete copy per type anyway, futures and
all.

It breaks for `dyn Trait`. A trait object needs a **vtable** — one
fixed-size slot per method, valid across every possible implementor. If
`Fetcher::run` and `Cruncher::run` each return a differently-sized,
differently-shaped anonymous future type, there's no single fixed-size
slot that fits both. The compiler catches this directly:

```
error[E0038]: the trait `Worker` is not dyn compatible
              ...because method `run` is `async`
```

### The `async-trait` macro's fix

```rust
#[async_trait::async_trait]
trait Worker {
    async fn run(&self) -> String;
}

#[async_trait::async_trait]
impl Worker for Fetcher {
    async fn run(&self) -> String { format!("hello from Fetcher") }
}
```

The macro rewrites the signature to roughly:

```rust
fn run(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + '_>>
```

— an ordinary, non-async method that returns an **already-boxed** future.
Boxing erases the size difference between implementors (a `Box` is always
one pointer, regardless of what future it points to), which restores
dyn-compatibility. `Vec<Box<dyn Worker>>` now compiles, and calling
`.run().await` on a trait object works exactly as it does on a concrete
type — the macro's rewrite is invisible at the call site.

This is why a trait with async methods, meant to back a
`Vec<Box<dyn Trait>>` across many differently-shaped implementors, needs
`#[async_trait::async_trait]` on both the `trait` block and every `impl`
block — not just one or the other.

---

## Recap

- A `trait` is a compile-time-checked contract: method signatures a type
  promises to implement, via an explicit `impl Trait for Type` block — no
  duck typing, the compiler verifies every required method exists with the
  right signature before the program runs.
- A trait method can have a **default** body; an `impl` that doesn't
  override it inherits the default, one that does override it hides the
  default entirely for that type.
- **Generic bound** (`fn f<T: Trait>`) — static dispatch. The compiler
  monomorphizes: a separate concrete copy of the function per type
  actually used, zero runtime dispatch cost, paid for in compile time and
  binary size.
- **Trait object** (`dyn Trait`, always behind a pointer — `&dyn Trait` or
  `Box<dyn Trait>`) — dynamic dispatch. Needed when the concrete type
  isn't known until runtime, e.g. mixing different concrete types in one
  `Vec<Box<dyn Trait>>`. Costs one vtable pointer-chase per call; see
  [05a-box-vec-dyn.md](05a-box-vec-dyn.md) for the full mechanics of how
  `Box`, `Vec`, and `dyn` compose.
- Plain `async fn` in a trait works for generic bounds but breaks
  dyn-compatibility (`E0038`), because each implementor's future is a
  differently-shaped anonymous type. `#[async_trait::async_trait]` fixes
  it by boxing the future, giving every implementor's method the same
  fixed-size return type.

<!-- sections below added as later exercises are completed -->
