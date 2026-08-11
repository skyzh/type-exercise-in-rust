# Chapter 12: Strengthen Rust Type Boundaries

{{#include wip-banner.md}}

The engine's runtime behavior is complete. This chapter makes its Rust ownership and sharing claims
executable without changing expression results.

**Prerequisites:** Chapter 11, `Any`, trait objects, threads, and lifetime variance.

**By the end of this chapter, you will:**

- return opaque borrowed array iterators;
- recover concrete expressions through checked `Any` downcasts; and
- prove `Send + Sync`, captured-state, and lifetime-shortening boundaries.

```console
cargo x copy-test --chapter 12
cargo test -p type-exercise-starter chapter_12 --locked
```

The first run should fail on one or more iterator, trait-object, thread, or lifetime guarantees.

## Checkpoint 1: keep iterator storage private

- **Target:** `type-exercise-starter/src/array.rs::Array::iter` and
  `type-exercise-starter/src/array/iterator.rs::ArrayIterator`.
- **Change:** return `impl Iterator<Item = Option<Self::RefItem<'_>>>` while preserving borrowed
  strings and nullable rows.
- **Preserve:** callers do not name the concrete iterator type.
- **Run:** the Chapter 12 focused test and doctests.
- **Passing means:** integer and string iteration keep their Chapter 1 ownership behavior.

## Checkpoint 2: recover a concrete expression safely

- **Target:** `type-exercise-starter/src/expression.rs::Expression: Any + Send + Sync`.
- **Change:** upcast a trait object to `dyn Any` and use `downcast_ref` for checked recovery.
- **Preserve:** mismatches return `None`; never cast raw pointers.
- **Run:** the focused test.
- **Passing means:** erased objects can be inspected without weakening object safety.

## Checkpoint 3: prove sharing and variance

- **Target:** `type-exercise-starter/src/binder.rs::FunctionRegistry::{register, register_unary,
  register_binary, register_ternary}` and
  `type-exercise-starter/src/column.rs::ColumnViewImpl<'a>`.
- **Change:** require captured factories to be `Send + Sync + 'static` and demonstrate shortening a
  valid column borrow.
- **Preserve:** a borrow cannot be lengthened or escape its backing array.
- **Run:** focused, cumulative, and compile-fail doctests.
- **Passing means:** expressions and registries can be shared across worker threads while views
  remain tied to their data.

## Required and extension work

Opaque iteration, checked recovery, thread-safety, and covariance are required. Unsafe downcasts,
custom executors, and arbitrary lifetime conversion helpers are not.

```console
cargo test -p type-exercise-starter chapter_12 --locked
cargo test -p type-exercise-starter --doc --locked
cargo test -p type-exercise-starter --lib --locked
```

{{#include copyright.md}}
