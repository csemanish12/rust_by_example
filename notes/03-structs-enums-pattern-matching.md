# 03 — Structs, Enums, and Pattern Matching

**Covers:** `struct` + `impl` blocks, constructing and calling methods on a
struct, `enum` with unit/tuple/struct-style variants, exhaustive `match`,
and how `for` loops relate back to move/borrow semantics from topic 2.

---

## Structs — grouped, named data

```rust
struct Employee {
    name: String,
    age: u32,
}
```

Closest Python equivalent: a dataclass. Every field has an explicit type —
no inference on struct fields, same rule as function parameters.

Methods are **not** defined inside the struct body — they live in a
separate `impl` (implementation) block:

```rust
impl Employee {
    fn describe(&self) -> String {
        format!("{} is {} years old", self.name, self.age)
    }
}
```

- `impl Employee { ... }` attaches methods to the struct; the struct
  definition itself stays pure data.
- `&self` — this method only reads the struct's fields, so it borrows
  itself (shared borrow) rather than taking ownership. Same `&` rules from
  topic 2 apply to `self` exactly like any other parameter.
- `format!` is a macro like `println!`, but it returns a `String` instead
  of printing to stdout.

Constructing and using one:

```rust
fn main() {
    let emp = Employee {
        name: String::from("Alex"),
        age: 30,
    };

    println!("{}", emp.describe());
}
```

- `Employee { name: ..., age: ... }` — every field is named explicitly at
  construction time. No positional constructor, unlike a Python dataclass.
- `emp.describe()` — `.` calls a method or accesses a field on a value.
  (`::`, from topic 1, is for reaching into a module/type's namespace —
  `.` and `::` are not interchangeable.)

---

## Enums — more than a fixed set of names

A basic enum looks like Python's `Enum`:

```rust
enum TrafficLight {
    Red,
    Yellow,
    Green,
}
```

But Rust enum variants can carry their own data, which is where this
diverges hard from Python's version. Three variant styles exist:

```rust
enum Example {
    Nothing,                             // unit variant — no data (Red, Green above)
    OneValue(u32),                       // tuple variant — one unnamed field
    TwoValues(String, u32),              // tuple variant — multiple unnamed fields
    Named { name: String, count: u32 },  // struct-style variant — named fields
}
```

Any type can be carried: primitives, `String`, `Vec<T>`, `Option<T>`,
other structs/enums, even `Box<Self>` for a recursive shape.

Variant names follow the same identifier rules as struct/type names:
letters/digits/underscores only, no spaces, `PascalCase` by convention
(`FlashingYellow`, not `flashing_yellow`) — a variant really is its own
little type-shape, hence the type-style naming.

---

## `match` — exhaustive pattern matching

```rust
fn instruction(light: &TrafficLight) -> String {
    match light {
        TrafficLight::Red => String::from("Stop"),
        TrafficLight::Yellow => String::from("Slow down"),
        TrafficLight::Green => String::from("Go"),
    }
}
```

- `match` compares `light` against each pattern in turn and runs the first
  arm that matches.
- **No `default`/`else` needed or allowed to be skipped** — `match` forces
  you to cover every possible variant. This is enforced at compile time.
- Arm order doesn't matter for correctness *when patterns don't overlap*
  (plain enum variants never overlap — a value is always exactly one
  variant). Order only matters once patterns can overlap (e.g. a numeric
  range next to a wildcard `_`), since `match` uses the *first* arm that
  matches, top to bottom.

### Extracting data from a tuple variant

To pull the carried data out of a tuple-style variant, bind it to a name
directly in the pattern:

```rust
enum TrafficLight {
    Red,
    Yellow,
    Green,
    FlashingYellow(u32),
}

fn instruction(light: &TrafficLight) -> String {
    match light {
        TrafficLight::Red => String::from("Stop"),
        TrafficLight::Yellow => String::from("Slow down"),
        TrafficLight::Green => String::from("Go"),
        TrafficLight::FlashingYellow(seconds) => format!("Flashing yellow, {seconds}s left"),
    }
}
```

`seconds` here is a new local variable, bound to whatever `u32` value the
matched `FlashingYellow` instance is carrying.

### The exhaustiveness check catching a real bug class

Adding `FlashingYellow(u32)` to the enum **without** updating the `match`
produces a compile error:

```
error[E0004]: non-exhaustive patterns: `&TrafficLight::FlashingYellow(_)` not covered
 --> src/bin/03_enums_1.rs:9:11
  |
9 |     match light {
  |           ^^^^^ pattern `&TrafficLight::FlashingYellow(_)` not covered
```

This is the exhaustiveness guarantee doing real work: the compiler knows
every variant an enum can ever have, and the moment a new variant is added,
every existing `match` that doesn't account for it becomes a compile
error — not a silent runtime bug. This directly prevents a common class of
bug in other languages: add a new status/case value, forget to update one
of several `if`/`elif` chains that switch on it somewhere in the codebase,
ship a silent gap. Rust finds every place that needs updating for you,
immediately, at compile time.

---

## `for` loops are move/borrow semantics in disguise

Iterating over a collection is a natural place to accidentally trigger the
exact same move rules from topic 2, since a `for` loop desugars to calling
`.into_iter()` on whatever follows `in`:

```rust
let lights = [TrafficLight::Red, TrafficLight::Yellow, TrafficLight::Green];

for light in lights {         // moves `lights`, one element out per iteration
    println!("{}", instruction(&light));
}

println!("{}", lights.len()); // ERROR: `lights` was moved into the loop
```

```
error[E0382]: borrow of moved value: `lights`
  |
  = note: `into_iter` takes ownership of the receiver `self`, which moves `lights`
help: consider iterating over a slice of the `[TrafficLight; 3]`'s content to avoid moving into the `for` loop
  |
  for light in &lights {
```

Versus borrowing the collection instead:

```rust
for light in &lights {                  // borrows; `lights` stays valid after
    println!("{}", instruction(light)); // `light` is already a &TrafficLight
}

println!("{}", lights.len()); // fine — lights was never moved
```

- `for light in lights` (no `&`) — calls `.into_iter()` on the owned array,
  which **moves** each element out. `lights` is consumed; using it again
  after the loop is a compile error.
- `for light in &lights` — calls `.into_iter()` on the *reference*, which
  yields a `&TrafficLight` per element instead of moving it out. `lights`
  remains valid and usable after the loop.
- Because `light` is already `&TrafficLight` in the borrowing version,
  `instruction(light)` needs no extra `&` at the call site — it already
  matches the function's `&TrafficLight` parameter.

`for x in &collection` is the idiomatic default when you just need to read
each item without consuming the collection.

---

## `Option<T>` and `Result<T, E>` are just enums

Nothing about them is special language magic — they're ordinary enums,
defined in the standard library, that anyone could have written themselves:

```rust
enum Option<T> {
    Some(T),
    None,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

(`<T>`/`<E>` are generics — a placeholder for "some type," filled in at each
use site. Covered properly in a later topic; for now, read `Option<T>` as
"either `Some` holding a value of type `T`, or nothing at all.")

Their variants are tuple-style (`Some(T)`, `Ok(T)`, `Err(E)`) or unit
(`None`) — the exact same shapes as `FlashingYellow(u32)` and `Red` from
the `TrafficLight` enum earlier in this file. They match the same way too.

### `Option<T>` — value, or nothing

```rust
fn safe_divide(a: f64, b: f64) -> Option<f64> {
    if b == 0.0 {
        None
    } else {
        Some(a / b)
    }
}

fn main() {
    let results = [safe_divide(10.0, 2.0), safe_divide(5.0, 0.0)];

    for result in &results {
        match result {
            Some(value) => println!("Result: {value}"),
            None => println!("Cannot divide by zero"),
        }
    }
}
```

The function's own return type, `Option<f64>`, tells every caller up front
"this might not produce a value" — there's no implicit `null`/`None`
sneaking through untyped like in Python. The possibility of absence is part
of the type signature, and `match` forces you to handle both cases.

### `Result<T, E>` — value, or a specific error explaining why not

```rust
fn safe_divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err(String::from("cannot divide by zero"))
    } else {
        Ok(a / b)
    }
}

fn main() {
    let results = [safe_divide(10.0, 2.0), safe_divide(5.0, 0.0)];

    for result in &results {
        match result {
            Ok(value) => println!("Result: {value}"),
            Err(message) => println!("Error: {message}"),
        }
    }
}
```

Compared to `Option<T>`: `Option` just says "value or nothing," with no
explanation for the "nothing" case. `Result<T, E>` says "value, **or** a
specific error explaining what went wrong" — the `E` slot carries that
reason. Here it's a plain `String` message; a later topic (error handling)
covers giving it a proper structured error type instead of a raw string.

---

## The prelude — why some things never need a `use`

`Option`, `Result`, and their variants (`Some`, `None`, `Ok`, `Err`) never
needed a `use` statement anywhere in this file. That's because they're part
of Rust's **prelude** — a small, fixed set of items the compiler
automatically brings into scope in *every* Rust file, with no import
required. Closest Python analogy: builtins like `print`/`len`/`str`, which
are always available without an `import`. The prelude is deliberately kept
small; almost everything else in Rust needs an explicit `use`.

Two separate things are actually happening for `Option`/`Result`
specifically:
1. `Option`/`Result` themselves don't need `use std::option::Option;` —
   the *types* are in the prelude.
2. `Some`/`None`/`Ok`/`Err` don't need writing as `Option::Some`/
   `Result::Ok` — the prelude re-exports the *variants* directly too,
   purely as a convenience since they're used so often. Writing
   `Option::Some(5)` still works, it's just more verbose.

Contrast with a custom enum like `TrafficLight`: nothing about it is in any
prelude, so its variants always need the full `TrafficLight::Red` path
(or an explicit `use TrafficLight::*;` to bring them into scope the way
`Option`'s are, which is uncommon style for your own enums).

`Option`/`Result` aren't the only prelude residents — other things you'll
meet soon that also never need a `use`:
- Common types: `String`, `Vec<T>`, `Box<T>`
- Common traits: `Clone`, `Copy` (from topic 2), `Drop`, `Debug`
  (implemented via `#[derive(...)]` on your own structs/enums, e.g.
  `#[derive(Debug)]`)
- `println!`/`format!`/`vec!` and the other commonly used macros

If you ever see an unfamiliar type or trait used with **no** matching `use`
line anywhere in a file, the prelude is the first thing to suspect — the
alternative is that it's a plain function/type defined earlier in the same
file, which you'd be able to find by scrolling up.

---

## Recap

- `struct` groups named, typed fields; methods live in a separate `impl`
  block, called with `.` on an instance.
- `enum` variants can be unit (no data), tuple-style (positional data), or
  struct-style (named data) — not just a fixed set of labels like Python's
  `Enum`.
- `match` must cover every variant of an enum — the compiler enforces this,
  turning "forgot to handle a new case" into a compile error instead of a
  silent bug.
- Pattern-matching a tuple variant binds its carried data to a new local
  name directly in the match arm.
- `for x in collection` moves; `for x in &collection` borrows — the same
  ownership rules from topic 2 apply inside loops, via `.into_iter()`.
- `Option<T>` (`Some`/`None`) and `Result<T, E>` (`Ok`/`Err`) are ordinary
  enums from the standard library, not language magic — `Option` encodes
  "value or nothing" in the type signature itself; `Result` encodes "value
  or a specific error." Both are in the prelude, so neither they nor their
  variants need a `use` statement.
