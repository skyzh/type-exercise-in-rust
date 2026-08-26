# Chapter 10: Specialize One Primitive Loop

{{#include wip-banner.md}}

Correct generic evaluation comes first. Now you can optimize one common case and prove that every
other representation still follows the established path.

**Prerequisites:** Chapter 9 and basic benchmarking discipline.

**By the end of this chapter, you will:**

- select an all-valid `i32` loop once per batch;
- preserve the general path for nulls, dictionaries, and other operators; and
- compare loop shapes without moving validation into the measured row loop.

```console
cargo x copy-test --chapter 10
cargo test -p type-exercise-starter chapter_10 --locked
```

The first run should fail on fast-path selection while the earlier general evaluator stays green.

## Prove the fast-path preconditions

A checked `ColumnViewImpl::try_non_null_array` proves once that an array has no null rows and
records `Nullability::NonNull` beside its `PhysicalType`. A constant value is already one non-null
scalar repeated for the batch. These facts permit four dense binary loop shapes over the same
primitive array representation:

- array / array;
- array / constant;
- constant / array; and
- constant / constant.

A dictionary, typed null, nullable array, type mismatch, arity mismatch, or length mismatch must
return to the general contract or the same structured error.

## Checkpoint 1: select once per batch

- **Target:** `type-exercise-starter/src/physical_type.rs::Nullability`,
  `type-exercise-starter/src/column.rs::{ColumnViewImpl::nullability,
  ColumnViewImpl::try_non_null_array}`, and
  `type-exercise-starter/src/expression.rs::{Expression::output_nullability, PrimitiveLoop,
  PrimitiveBinaryExpression::evaluate_with_loop}`.
- **Change:** keep one primitive array representation, establish physical nullability at the
  column boundary, and choose the dense `i32` path only after ordinary validation succeeds.
- **Preserve:** output values, nulls, and errors are identical to `evaluate`.
- **Run:** the Chapter 10 focused test.
- **Passing means:** all four dense shapes report their selected loop and every fallback reports
  `PrimitiveLoop::General`.

## Checkpoint 2: forward through binding

- **Target:** `type-exercise-starter/src/binder.rs::{BoundExpression::output_nullability,
  BoundExpression::evaluate_with_loop}`.
- **Change:** delegate nullability propagation and evaluation to the already-selected physical
  expression.
- **Preserve:** logical selection does not choose a fast path; batch representation does.
- **Run:** focused and cumulative tests.
- **Passing means:** binding and non-primitive catalog entries remain unchanged.

## Required and extension work

Representative `i32` specialization and semantic fallbacks are required. Fast paths for every
numeric family and operator are extensions. Do not duplicate the full evaluator to chase a
benchmark.

```console
cargo test -p type-exercise-starter chapter_10 --locked
cargo test -p type-exercise-starter --lib --locked
```

After the tests pass, you may run the maintained reference benchmark without reading its source:

```console
cargo bench -p type-exercise --bench expression
```

It reports the four dense shapes and three fallbacks separately. Setup and dictionary validation
stay outside the timed row loop. The measurements are machine-specific observations, not a
completion gate.

Next: [Chapter 11 builds a one-level List column](./chapter-11-list.md).

{{#include copyright.md}}
