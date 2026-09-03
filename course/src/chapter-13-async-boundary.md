# Chapter 13: Share Logical Factories Across Threads

The completed Day 12 engine already preserves five Rust boundaries that matter here: array
iterators borrow their arrays behind an opaque return type, erased expressions support checked
`Any` recovery and are safe to share, and erased column views can shorten their borrow for one
evaluation call. This chapter keeps those compiler-enforced properties intact. Your one new change
is to make the logical function registry safe to share when its factories capture state.

## See the missing bound

```console
cargo x copy-test --chapter 13
cargo test -p type-exercise-starter-supplied-tests chapter_13 --locked
```

The first run is compile-red only at the captured-factory test: `FunctionRegistry` stores a trait
object that is not yet `Send + Sync`, so the registry cannot cross a thread boundary. The other
five tests protect properties already present at the end of Day 12; they are regression witnesses,
not additional implementation tasks.

## Strengthen the factory boundary

In `expr/src/binder.rs`, require the stored `FunctionFactory` and every factory accepted by
`register`, `register_unary`, `register_binary`, and `register_ternary` to be
`Send + Sync + 'static`. Keep the callable bound as `Fn`: registration stores a reusable factory,
and shared calls must not require mutable access to the closure itself.

A factory can still capture state through thread-safe shared ownership. The supplied test uses an
`Arc<AtomicUsize>` to observe one call after the registry crosses a worker-thread boundary. Do not
add a marker trait, expose a new helper, use `unsafe`, or turn borrowed evaluation data into
`'static` data.

## Inspect the inherited boundaries

No core-code edit is required for the other five tests:

- `Array::iter` already returns an opaque `impl Iterator + 'a`; integer rows stay nullable and
  string items remain borrowed from the array. Its concrete implementation is private and is not
  part of the learner contract.
- `Expression` already extends `Any + Send + Sync`, so a `dyn Expression` can upcast directly to
  `dyn Any` for checked recovery and can be shared with a worker thread.
- `ColumnViewImpl<'a>` already stores borrows covariantly, so a longer-lived view can be reborrowed
  for a shorter expression call without `unsafe` or a `'static` workaround.

Run the focused and cumulative contracts:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_13 --locked
cargo test -p type-exercise-starter-supplied-tests --locked
cargo test -p type-exercise-starter-expr --doc --locked
cargo test -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The focused result is 6 passed, and the cumulative result is 111 passed. Together they prove the
one new thread-safe factory boundary without regressing the five inherited borrowing, erasure, and
sharing properties.

The result is still the same synchronous expression engine. The stronger ownership boundary is
what lets Chapter 14 borrow expressions and input views across one future without cloning their
contents.

Next: [Chapter 14 adds a batch async boundary](./chapter-14-async-boundary.md).

{{#include copyright.md}}
