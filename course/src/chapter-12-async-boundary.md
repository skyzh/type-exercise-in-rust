# Chapter 12: Add a Batch Async Boundary

Some engines need a uniform asynchronous interface even when a local expression is immediately
ready. The useful boundary is one future per batch, not one future per row.

**Prerequisites:** Chapter 11, `Future`, `Pin`, and return-position `impl Trait`.

**By the end of this chapter, you will:**

- wrap static expression evaluation in a borrowed batch future;
- erase that future behind an object-safe asynchronous interface; and
- preserve every synchronous result and error without evaluating twice.

```console
cargo x copy-test --chapter 12
cargo test -p type-exercise-starter chapter_12 --locked
```

The first run should fail on the missing static or erased batch future boundary.

## Checkpoint 1: return one static future

- **Target:** `type-exercise-starter/src/expression.rs::evaluate_static`.
- **Change:** return `impl Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a` that
  borrows the expression and input slice.
- **Preserve:** the body delegates to the existing synchronous batch evaluation exactly once.
- **Run:** the Chapter 12 focused test.
- **Passing means:** static sync and async paths return identical arrays and errors.

## Checkpoint 2: erase the future

- **Target:** `type-exercise-starter/src/expression.rs::{BatchFuture, AsyncExpression, AsyncExpressionAdapter}`.
- **Change:** box and pin the borrowed batch future so it can be returned from a trait object.
- **Preserve:** the future lifetime covers the expression, the view slice, and every borrowed
  backing array.
- **Run:** the focused test.
- **Passing means:** erased async evaluation matches static and synchronous evaluation.

## Checkpoint 3: forward a bound plan

- **Target:** `type-exercise-starter/src/binder.rs::BoundExpression::evaluate_async`.
- **Change:** delegate to the already-selected expression without repeating logical binding.
- **Preserve:** arity, type, length, null, and scalar errors keep the same variants and precedence.
- **Run:** focused and cumulative tests.
- **Passing means:** the planning boundary remains one-time and the batch kernel remains synchronous.

## Required and extension work

One ready future per batch is required. I/O, timers, retries, background threads, cancellation
protocols, custom runtimes, and per-row futures are outside this course.

```console
cargo test -p type-exercise-starter chapter_12 --locked
cargo test -p type-exercise-starter --lib --locked
```

You have now moved type selection, representation dispatch, validation, promotion, and runtime
selection out of the row loop while keeping each failure boundary explicit.

{{#include copyright.md}}
