# Chapter 5: Make Numeric Evaluation Generic

The type family now earns its cost: one binary adapter can evaluate four arithmetic operations
across every supported, lossless promotion row.

**Prerequisites:** Chapters 1–4 and integer/float edge cases.

**By the end of this chapter, you will:**

- select a common numeric output from one ordered promotion table;
- run `+`, `-`, `*`, and `/` through the same checked binary path; and
- report row-attributed division failures without evaluating strict null rows.

```console
cargo x copy-test --chapter 5
cargo test -p type-exercise-starter chapter_5 --locked
```

The first run should fail on the promotion table and generic arithmetic catalog.

## Make promotion explicit

`NUMERIC_PROMOTIONS` is an ordered-pair table, not a Rust `as` policy. Both operand orders must be
present when supported. The initial course accepts widening rows such as `i16 → i32`, `i32 → i64`,
and `f32 → f64`; it rejects pairs such as `i64` with `f64` because that conversion is not lossless.

`Decimal` has a physical family but no arithmetic row. Precision, scale, rounding, overflow, and
division scale need a separate contract before Decimal arithmetic is safe to teach.

## Checkpoint 1: pin the promotion matrix

- **Target:** `type-exercise-starter/src/promotion.rs::{NumericPromotion, NUMERIC_PROMOTIONS, promote_numeric}`.
- **Change:** return one common logical output for every approved ordered pair.
- **Preserve:** unsupported and lossy pairs return `None`; duplicated or substituted rows fail.
- **Run:** the Chapter 5 focused test.
- **Passing means:** output type selection is symmetric where intended and explicit everywhere.

## Checkpoint 2: implement generic arithmetic kernels

- **Target:** `type-exercise-starter/src/operators.rs::{ArithmeticOperator, CheckedBinaryExpression,
  build_numeric_binary_expression}`.
- **Change:** dispatch once to the promoted output scalar and evaluate all four operations.
- **Preserve:** signed `+`, `-`, and `*` wrap explicitly. Integer division by zero and `MIN / -1`
  return `ScalarError`; `±0.0` divisors return division-by-zero, while other IEEE NaN/infinity
  results propagate.
- **Run:** the focused and cumulative tests.
- **Passing means:** operation semantics and output families agree for arrays and repeated values.

The first failing non-null row becomes
`ExpressionError::ScalarEvaluation { function, row, error }`. If any strict input is null, the row
is null and the scalar operation is not called; `null / 0` is therefore null, not an error.

The numeric builder chooses one typed shell from the promotion result. Logical function names and
the registry do not enter this path yet; Chapter 8 will bind them after Chapter 7 establishes the
runtime boundary.

## Required and extension work

The pinned matrix and all four operations are required. Decimal arithmetic, narrowing casts, and
precision-losing implicit casts are extensions only after their semantics are specified. Do not
broaden the table merely to make more Rust conversions compile.

```console
cargo test -p type-exercise-starter chapter_5 --locked
cargo test -p type-exercise-starter --lib --locked
```


{{#include copyright.md}}
