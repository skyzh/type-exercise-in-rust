{{#include wip-banner.md}}

# Chapter 13: Strengthen Rust Type Boundaries

The engine now has borrowed views, erased expressions, registries, and List slices. This chapter
does not add a new database feature. It makes the Rust contracts around those existing values
explicit so the compiler can preserve borrowing and thread-safety across generic and erased code.

## The learner-owned boundary

```console
cargo x copy-test --chapter 13
cargo test -p type-exercise-starter-expr chapter_13 --locked
```

You will strengthen three connected surfaces.

### Return opaque borrowed iterators

Make `Array::iter` return `impl Iterator` while retaining the lifetime that borrows its array. The
caller can traverse nullable fixed-width and string rows without naming the private iterator type
or allocating owned strings. Empty arrays remain ordinary empty iterators.

### Recover erased expression objects safely

Keep `Expression: Any + Send + Sync`, then support checked recovery and direct trait-object
upcasting where the language permits it. The erased object must be safe to share with worker
threads because its selected kernel and metadata are immutable for evaluation.

### Preserve captures and shorten borrows

Allow logical factories to capture thread-safe shared state. Keep `ColumnViewImpl<'a>` covariant
so a view with a longer borrow can be used for a shorter evaluation scope. Do not use `unsafe` or
erase the view lifetime to `'static`; the type relationship should follow from the data each enum
variant actually contains.

Run the full contract:

```console
cargo test -p type-exercise-starter-expr chapter_13 --locked
cargo test -p type-exercise-starter-expr --doc --locked
cargo test -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The six focused tests prove opaque iteration over integers and borrowed strings, checked
trait-object recovery, cross-thread expression use, captured shared state, and lifetime shortening
for erased views. These are compile-time API properties exercised through runnable values, not a
parallel hierarchy of marker types.

The result is still the same synchronous expression engine. The stronger ownership boundary is
what lets Chapter 14 borrow expressions and input views across one future without cloning their
contents.

Next: [Chapter 14 adds a batch async boundary](./chapter-14-async-boundary.md).

{{#include copyright.md}}
