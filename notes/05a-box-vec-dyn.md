# 05a — `Box`, `Vec`, and `dyn`: quick reference

**Covers:** what each of these three actually is, why each exists, and how
they compose (`Vec<Box<dyn Trait>>`). Pulled out of the trait-objects
exercise as a standalone reference since these three show up constantly
outside of traits too.

---

## `Vec<T>` — growable, contiguous, homogeneous

A `Vec<T>` is a growable array whose elements live **contiguously** in one
heap-allocated buffer. "Contiguous" is the load-bearing detail: to find
element `i`, Rust computes `base_pointer + i * size_of::<T>()` — pure
arithmetic, no searching. That only works if every element is the exact
same size, which means every element must be the same concrete type `T`,
with a size known at compile time.

```rust
let mut nums: Vec<i32> = Vec::new();
nums.push(1);
nums.push(2);
```

- `Vec` **owns** everything inside it — when the `Vec` is dropped, every
  element is dropped too (no manual free, same RAII pattern as everything
  else in Rust).
- `.iter()` yields `&T` per element — **borrows**, `Vec` keeps ownership.
- `.into_iter()` yields `T` per element — **consumes** the `Vec`, moves
  each element out. (You'll hit this distinction again in topic 7,
  closures & iterators.)
- Because every element must be one fixed-size `T`, `Vec<Person>` and
  `Vec<Robot>` each work fine alone — but there is no single `T` that
  lets you push both a `Person` and a `Robot` into the *same* `Vec`.

## `Box<T>` — a single owned heap allocation

`Box<T>` puts exactly one value on the heap and owns it. The `Box` value
you actually hold (the "handle") is always the same size — one pointer —
**regardless of how big or small `T` is**. That's the entire trick `Box`
offers: it converts "a value of unknown/variable size" into "a fixed-size
handle to that value."

```rust
let boxed: Box<i32> = Box::new(5);
```

- `Box::new(value)` moves `value` onto the heap; the `Box` now owns it.
- When the `Box` goes out of scope, its `Drop` impl frees the heap memory
  automatically — no manual `free`.
- Two situations specifically *require* `Box` (or another
  pointer/indirection type) rather than just being a style choice:
  1. **Recursive types** — a struct that contains itself (e.g. a linked
     list node holding "the next node") has no fixed size unless one of
     the self-references goes through a pointer.
  2. **Trait objects** — see `dyn Trait` below; `dyn Trait` alone has no
     fixed size, so it must always sit behind a pointer.

## `dyn Trait` — a trait object, size-erased

`dyn Greeter` doesn't name one concrete type — it names "whichever
concrete type implements `Greeter`, decided at runtime." Because different
implementors (`Person`, `Robot`, ...) can be different sizes, `dyn Greeter`
itself has **no fixed size** (Rust's term: *unsized*, `?Sized`). An
unsized value can never sit directly on the stack or directly inside a
`Vec` slot — it must always be accessed through some kind of pointer:
`&dyn Greeter` (borrowed), `Box<dyn Greeter>` (owned, heap), or later
`Rc<dyn Greeter>`/`Arc<dyn Greeter>` (shared ownership — topic 8).

Whichever pointer wraps it, a reference to a trait object is a **fat
pointer** — two words instead of one:
1. a pointer to the concrete value's actual data
2. a pointer to a **vtable** — a compiler-generated table of function
   pointers for that concrete type's implementation of the trait's methods

Calling a method through `&dyn Greeter`/`Box<dyn Greeter>` follows the
vtable pointer **at runtime** to find the right function — this is
**dynamic dispatch**, as opposed to the **static dispatch** you get from
plain generics (`fn f<T: Greeter>`), where the compiler generates a
separate monomorphized copy per concrete type at compile time and there's
no runtime lookup at all.

## Putting them together: `Vec<Box<dyn Greeter>>`

```rust
let mut greeters: Vec<Box<dyn Greeter>> = Vec::new();
greeters.push(Box::new(person));  // Person -> heap, wrapped in a fixed-size Box handle
greeters.push(Box::new(robot));   // Robot  -> heap, wrapped in a fixed-size Box handle
```

- The outer `Vec` needs one fixed element size — satisfied, because every
  element is a `Box` (one pointer), never the underlying `Person`/`Robot`
  directly.
- Each `Box` independently points to a different concrete type's data, on
  the heap, alongside its own vtable pointer.
- This is the actual mechanism for "a collection of genuinely different
  types that share a trait": impossible with `Vec<T>` alone (needs one
  concrete `T`), impossible with `dyn Greeter` alone (no fixed size),
  solved by combining both.

## Quick decision table

| Need | Use |
|---|---|
| One value, single owner, concrete type known | `Box<T>` (or just `T` on the stack, if it doesn't need to outlive its scope or be recursive/unsized) |
| Many values, all the same concrete type, growable | `Vec<T>` |
| "Any type implementing this trait," type fixed at compile time, one call site | generic bound `fn f<T: Trait>` — static dispatch |
| "Any type implementing this trait," type chosen at runtime, or genuinely mixed types in one collection | `dyn Trait`, always behind a pointer (`&dyn Trait`, `Box<dyn Trait>`) — dynamic dispatch |
