# 02 — Ownership & Borrowing

**Covers:** what "ownership" means and why Rust needs it, move semantics,
the `Copy` trait vs. move types, `.clone()` and its cost, and reading a
borrow-checker compile error.

*(In progress — this file is being built up alongside the exercises, not
written after the fact.)*

---

## Why ownership exists

Python has a garbage collector: you never think about when memory gets
freed, it just happens whenever nothing references a value anymore. Rust has
**no garbage collector**. Instead, the compiler enforces a rule at compile
time: every value has exactly **one owner**, and the value is freed the
instant that owner goes out of scope. This rule is what lets Rust manage
memory safely without a GC pausing your program at runtime.

"Ownership" is just the name for the rules that keep that "exactly one
owner" invariant true.

## Move semantics

```rust
let s1 = String::from("hello");
let s2 = s1;

println!("{s1}"); // does NOT compile
println!("{s2}");
```

`String` is a heap-allocated, **owned** type (closest Python equivalent:
`str`, but Python never tracks a single owner — Rust does). Writing
`let s2 = s1;` does **not** copy the string data. It **moves** ownership from
`s1` to `s2`. After that line, `s1` is no longer valid — the compiler treats
it as dead, and using it again is a compile error, not a runtime bug.

The actual compiler error:

```
error[E0382]: borrow of moved value: `s1`
 --> src/bin/01_ownership.rs:6:16
  |
2 |     let s1 = String::from("hello");
  |         -- move occurs because `s1` has type `String`, which does not implement the `Copy` trait
3 |     let s2 = s1;
  |              -- value moved here
...
6 |     println!("{s1}");
  |                ^^ value borrowed here after move
  |
help: consider cloning the value if the performance cost is acceptable
  |
3 |     let s2 = s1.clone();
  |                ++++++++
```

How to read it:
- `error[E0382]` — every rustc error has a numeric code you can get more
  detail on with `rustc --explain E0382`.
- The message points to **exactly** where the move happened (line 3) and
  exactly where the dead value was used again (line 6) — no guessing needed.
- It also suggests a fix directly in the error output (`s1.clone()`).

This is a compile-time check catching something Python simply has no concept
of — there, reassigning a variable never invalidates the old one.

## The `Copy` trait — why integers behave differently

```rust
let x = 5;
let y = x;

println!("{x}"); // this DOES compile
println!("{y}");
```

This compiles fine, with no move error, even though it looks structurally
identical to the `String` example. The difference: `i32`/`u64`/other small,
fixed-size primitive types implement a special trait called `Copy`. For
`Copy` types, `let y = x;` **copies** the value instead of moving it — both
`x` and `y` stay valid afterward.

`String` does not implement `Copy`, because it owns heap memory. If simple
assignment silently deep-copied heap data every time, that would be a hidden
performance cost — and Rust's design philosophy is "no hidden costs the
programmer didn't ask for." So types that own heap allocations (`String`,
`Vec<T>`, etc.) move by default; only cheap, fixed-size, stack-only types
(`i32`, `u64`, `bool`, `char`, etc.) implement `Copy`.

## `.clone()` — an explicit, deliberate deep copy

Fix for the move error, straight from the compiler's own suggestion:

```rust
let s1 = String::from("hello");
let s2 = s1.clone();

println!("{s1}"); // fine — s1 is untouched
println!("{s2}");
```

`.clone()` explicitly allocates new heap memory and copies the string's
contents, so `s1` and `s2` become two fully independent, valid values. This
compiles and both prints work.

The important trade-off: `.clone()` is a **real, visible cost** — a heap
allocation plus a full copy of the data. That's fine for a short string in a
learning exercise, but calling `.clone()` reflexively everywhere in a real
program to dodge move errors defeats a lot of the point of Rust's ownership
model. It's a legitimate tool, not the default instinct — the next section
(borrowing) is usually the better fix when you just need to *read* a value
without taking ownership of it.

---

## Borrowing — using a value without owning it

`.clone()` avoids the move error, but it has a real cost: a heap allocation
plus a full copy of the data. **Borrowing** is the actual idiomatic fix when
a function just needs to *read* (or briefly modify) a value — not take
ownership of it.

```rust
fn print_it(s: &String) {
    println!("{s}");
}

fn main() {
    let s1 = String::from("hello");

    print_it(&s1);
    print_it(&s1);
}
```

The parameter type `&String` (not `String`) and the call-site `&s1` (not
`s1`) are what make this a borrow instead of a move. `print_it` gets
**temporary, read-only access** to `s1` — the borrow ends when `print_it`
returns, and `s1` is still fully owned by `main`, unaffected. That's why
calling `print_it(&s1)` twice works with zero `.clone()`s and zero move
errors: nothing ever took ownership away from `s1` in the first place.

This `&T` form is called a **shared borrow**. Any number of shared borrows
of the same value can exist at once, because none of them can mutate it.

## `&mut T` — the mutable/exclusive borrow

To actually mutate a value through a borrow, use `&mut T`:

```rust
fn add_exclamation(s: &mut String) {
    s.push_str("!");
}

fn main() {
    let mut s1 = String::from("hello");
    add_exclamation(&mut s1);

    println!("{s1}"); // hello!
}
```

Two things worth noting:
- The variable itself must be declared `let mut s1`, before you're even
  allowed to take a mutable borrow of it — Rust won't let you mutably borrow
  something that isn't itself mutable.
- The call site (`&mut s1`) and the parameter type (`&mut String`) both say
  `mut` — this is an **exclusive** borrow, not a shared one.

## The exclusivity rule

Unlike shared borrows, only **one** mutable borrow of a value can exist at a
time, and it cannot coexist with any shared borrow of that same value:

```rust
fn main() {
    let mut s1 = String::from("hello");

    let r1 = &s1;
    let r2 = &mut s1;

    println!("{r1} {r2}");
}
```

```
error[E0502]: cannot borrow `s1` as mutable because it is also borrowed as immutable
 --> src/bin/02_borrowing_exclusivity.rs:5:14
  |
4 |     let r1 = &s1;
  |              --- immutable borrow occurs here
5 |     let r2 = &mut s1;
  |              ^^^^^^^ mutable borrow occurs here
6 |
7 |     println!("{r1} {r2}");
  |                -- immutable borrow later used here
```

**Why this rule exists:** a mutable reference lets you change data out from
under any other reference currently looking at it. If shared and mutable
borrows could coexist, one part of the code could read through `r1` while
another mutates the same memory through `r2` — a data race, or at minimum a
value changing unexpectedly mid-read. The rule — *many shared borrows, OR
exactly one mutable borrow, never both, for the same value* — makes that
whole class of bug a compile error instead of a runtime bug (or a threading
bug that only shows up occasionally).

Summary so far:
- `&T` — shared, read-only, any number allowed at once.
- `&mut T` — exclusive, read-write, only one allowed, and never alongside a
  `&T` of the same value.

## Non-lexical lifetimes (NLL) — borrows end at last use, not end of scope

A natural assumption is that a borrow lasts for its entire enclosing block.
That's not quite right, and the distinction matters:

```rust
fn main() {
    let mut s1 = String::from("hello");

    let r1 = &s1;
    println!("r1 is {r1}");   // last use of r1

    let r2 = &mut s1;
    println!("r2 is {r2}");
}
```

This **compiles fine** — no `E0502` — even though `r1` and `r2` both exist
within the same `main()` body. The reason: the compiler doesn't track a
borrow's lifetime as "declared until the block ends." It computes each
borrow's actual **live range**: from where it's created to its **last real
use** (a real use = the borrow is actually read or written somewhere, e.g.
passed to a function, formatted in a `println!`, compared, indexed, etc.).

Here, `r1`'s last use is the `println!` right after it's created. Nothing
later in the function ever reads through `r1` again, so the compiler proves
`r1`'s borrow has already ended by the time `r2` is created a few lines
down. No overlap in live ranges → no conflict. This behavior has a name:
**non-lexical lifetimes (NLL)** — worth remembering as a term, since it's
commonly referenced when people explain the borrow checker.

Proof this is really about *usage*, not *position in the file*: adding one
more use of `r1` *after* `r2` is created reintroduces the exact same error,
because now `r1`'s live range is forced to extend past `r2`'s creation:

```rust
fn main() {
    let mut s1 = String::from("hello");

    let r1 = &s1;
    println!("r1 is {r1}");

    let r2 = &mut s1;
    println!("r2 is {r2}");

    println!("r1 is again {r1}"); // <-- extends r1's live range
}
```

```
error[E0502]: cannot borrow `s1` as mutable because it is also borrowed as immutable
  --> src/bin/02_borrowing_nll.rs:7:14
   |
 4 |     let r1 = &s1;
   |              --- immutable borrow occurs here
...
 7 |     let r2 = &mut s1;
   |              ^^^^^^^ mutable borrow occurs here
...
10 |     println!("r1 is again {r1}");
   |                            -- immutable borrow later used here
```

The error message even points at the *new* trailing use as the reason the
immutable borrow is "later used" — direct confirmation that live range is
computed from actual use sites, not lexical scope.

---

## Recap

- Ownership: every value has exactly one owner; it's freed when that owner
  goes out of scope. No garbage collector needed.
- `let s2 = s1;` **moves** ownership for non-`Copy` types (like `String`) —
  `s1` becomes invalid afterward. Using it again is `error[E0382]`.
- `Copy` types (`i32`, `u64`, `bool`, `char`, ...) are copied on assignment
  instead of moved — both variables stay valid.
- `.clone()` is an explicit deep copy — a legitimate but real-cost fix for a
  move error, not the default reach-for-it tool.
- `&T` (shared borrow) lets code read a value without taking ownership —
  any number can exist at once.
- `&mut T` (mutable/exclusive borrow) allows mutation through the
  reference — only one can exist at a time, never alongside a `&T` of the
  same value. Violating this is `error[E0502]`.
- Borrow lifetimes are computed by actual last-use (non-lexical lifetimes),
  not by lexical block scope — a borrow "ends" as soon as nothing later in
  the code still needs it.
