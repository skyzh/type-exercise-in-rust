# Chapter 5: Make Numeric Evaluation Generic

The type family now earns its cost: one binary adapter can evaluate four arithmetic operations
and six numeric comparisons across every supported, lossless promotion row.

**Prerequisites:** Chapters 1–4 and integer/float edge cases.

**By the end of this chapter, you will:**

- select a common numeric output from one ordered promotion table;
- run `+`, `-`, `*`, and `/` through the same checked binary path;
- compare numbers with `<`, `<=`, `>`, `>=`, `=`, and `!=` on the same promoted family; and
- fail closed on division and comparison errors without evaluating strict null rows.

```console
cargo x copy-test --chapter 5
cargo test -p type-exercise-starter chapter_5 --locked
```

The first run should fail on the promotion table and generic arithmetic/comparison catalog.

## Make promotion explicit

`NUMERIC_PROMOTIONS` is an ordered-pair table, not a Rust `as` policy. Both operand orders must be
present when supported. The initial course accepts widening rows such as `i16 → i32`, `i32 → i64`,
and `f32 → f64`; it rejects pairs such as `i64` with `f64` because that conversion is not lossless.

`Decimal` has a physical family but no arithmetic row. Precision, scale, rounding, overflow, and
division scale need a separate contract before Decimal arithmetic is safe to teach.

## Checkpoint 1: pin the promotion matrix

- **Target:** `type-exercise-starter/src/promotion.rs::{NumericPromotion, NUMERIC_PROMOTIONS, promote_numeric}`.
- **Change:** return one common logical output for every approved ordered pair; the table and
  lookup work on logical `DataType`, and the physical family is derived with
  `DataType::physical_type()`.
- **Preserve:** unsupported and lossy pairs return `None`; duplicated or substituted rows fail.
- **Run:** the Chapter 5 focused test.
- **Passing means:** output type selection is symmetric where intended and explicit everywhere.

Wire the module like Chapter 4 did:

```rust,ignore
mod promotion;
pub use promotion::{NUMERIC_PROMOTIONS, NumericPromotion, promote_numeric};
```

Keep the public items in `promotion.rs`; this module wiring lets the copied test import them from
the starter crate root.

## Checkpoint 2: implement generic arithmetic kernels

- **Target:** `type-exercise-starter/src/operators.rs::{ArithmeticOperator, CheckedBinaryExpression,
  build_numeric_binary_expression}`.
- **Change:** dispatch once to the promoted output scalar and evaluate all four operations; the
  builder returns one checked shell per promoted family, evaluated with the Chapter 4 typed loop.
- **Preserve:** signed `+`, `-`, and `*` wrap explicitly. Integer division by zero and `MIN / -1`
  return a checked scalar error; `±0.0` divisors return division-by-zero, while other IEEE
  NaN/infinity results propagate.
- **Run:** the focused and cumulative tests.
- **Passing means:** operation semantics and output families agree for arrays and repeated values.

If any strict input is null, the row is null and the scalar operation is not called; `null / 0` is
therefore null, not an error. The batch evaluator returns an `Err` on the first failing non-null
row without producing partial output; the row-attributed error shape is chosen in Chapter 6.

## Checkpoint 3: compare through the same promotion table

- **Target:** `type-exercise-starter/src/operators.rs::{ComparisonOperator, build_numeric_comparison_expression}`.
- **Change:** promote both operands with the same lossless table used by arithmetic, then evaluate
  all six operators as one checked binary shell producing `Boolean` rows.
- **Preserve:** ordered comparisons with NaN are false, `=` is false, and `!=` is true; null rows
  stay null and never call the scalar comparison.
- **Run:** the focused and cumulative tests.
- **Passing means:** operand order, operator names, NaN behavior, and output families agree with
  the promotion table and the Chapter 4 null contract.

The numeric builders choose one typed shell from the promotion result. Logical function names and
the registry do not enter this path yet; Chapter 9 will bind them after Chapter 8 establishes the
runtime boundary.

## Required and extension work

The pinned matrix, all four operations, and all six comparisons are required. Decimal arithmetic,
narrowing casts, and precision-losing implicit casts are extensions only after their semantics are
specified. Do not broaden the table merely to make more Rust conversions compile.

```console
cargo test -p type-exercise-starter chapter_5 --locked
cargo test -p type-exercise-starter --lib --locked
```


{{#include copyright.md}}
