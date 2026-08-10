# Chapter 2: Scale the Physical Type Family

Adding `f64` beside the two Chapter 1 rows repeats physical variants, scalar variants, array
aliases, builders, and conversions. Repeating that work for every primitive makes drift more likely
than the type itself warrants.

**Prerequisites:** Chapter 1 and basic declarative macros.

**By the end of this chapter, you will:**

- add complete `i16`, `i64`, `bool`, `f32`, and `f64` static families plus a metadata-aware
  Decimal family;
- map planner-visible `DataType` values to physical storage; and
- keep the non-List physical rows in one catalog that drives exhaustive code.

```console
cargo x copy-test --chapter 2
cargo test -p type-exercise-starter chapter_2 --locked
```

Follow the `Day 2, checkpoint ...` comments beside the declarations in the named starter files.
The first focused run should fail at those `todo!` boundaries for the new families and catalog.

## Add Double explicitly before generalizing

Connect `DataType::Double`, `PhysicalType::Float64`, `f64`, `F64Array`, and its builder by hand.
Use the same checkpoints as Chapter 1. The copied test includes NaN, infinity, and signed zero so
the family cannot accidentally require total equality.

This explicit row is evidence: the repeated edits are real. Now a single family catalog can own
the remaining non-List rows:

| Logical type | Physical family | Owned / borrowed scalar |
| --- | --- | --- |
| `SmallInt` | `Int16` | `i16` / `i16` |
| `Integer` | `Int32` | `i32` / `i32` |
| `BigInt` | `Int64` | `i64` / `i64` |
| `Boolean` | `Bool` | `bool` / `bool` |
| `Real` | `Float32` | `f32` / `f32` |
| `Double` | `Float64` | `f64` / `f64` |
| `Varchar`, `Char` | `String` | `String` / `&str` |
| `Decimal(p, s)` | `Decimal(p, s)` | typed `Decimal` / typed `Decimal` |

`Char { width }` retains logical metadata without changing String storage. Decimal is different:
its precision and scale define the physical value's meaning, including for empty and all-null
arrays. Use `DecimalType::try_new(precision, scale)` and enforce `1 <= precision <= 38` plus
`scale <= precision`. This course keeps scale nonnegative.

## Checkpoint 1: make primitive storage generic

- **Target:** `type-exercise-starter/src/array/primitive_array.rs::{PrimitiveArray, PrimitiveArrayBuilder}`.
- **Change:** implement the `Array` family once for supported primitive scalars and expose aliases.
- **Preserve:** nullable, empty, and special-float behavior from Chapter 1.
- **Run:** the Chapter 2 focused test.
- **Passing means:** every primitive family satisfies the same reciprocal type equations.

## Checkpoint 2: add the family catalog

- **Target:** `type-exercise-starter/src/variant_catalog.rs::for_each_physical_family`, plus generated arms in
  `type-exercise-starter/src/physical_type.rs`, `type-exercise-starter/src/scalar.rs`, and `type-exercise-starter/src/array.rs`.
- **Change:** make one row define the physical variant, array, builder, owned scalar, and borrowed
  scalar.
- **Preserve:** `String` remains the one borrowed row and every downcast remains checked.
- **Run:** the focused test and inspect `PHYSICAL_FAMILY_CATALOG` failures.
- **Passing means:** omitting or duplicating a family becomes a compile or test failure.

## Checkpoint 3: map logical meaning to storage

- **Target:** `type-exercise-starter/src/data_type.rs::{DataType, DataType::physical_type}`.
- **Change:** add all scalar logical types in the table.
- **Preserve:** `DataType` is planner metadata; do not add `Nullable` or List yet.
- **Run:** the focused and cumulative tests.
- **Passing means:** every logical scalar type has one documented physical family.

## Checkpoint 4: give Decimal one shared descriptor

- **Target:** `type-exercise-starter/src/decimal.rs::{DecimalType, Decimal, DecimalError}` and
  `type-exercise-starter/src/array/decimal_array.rs::{DecimalArray, DecimalArrayBuilder}`.
- **Change:** store one flat `i128` unscaled coefficient per row, packed validity, and one checked
  `DecimalType` shared by the whole array. Require the descriptor before the first builder push.
- **Preserve:** null rows use validity rather than `Option<Decimal>` storage; empty and all-null
  arrays remain typed; a failed coefficient or metadata check must not append a partial row.
- **Run:** the focused and cumulative tests.
- **Passing means:** scalar, array, and erased Decimal values preserve exact precision and scale.

The represented value is `unscaled × 10^-scale`. A valid coefficient has fewer than or exactly
`precision` decimal digits, so `10^precision` itself is out of range. Validate with an
overflow-safe absolute value: `i128::MIN` is an ordinary error case, not a reason to panic. Do not
use `rust_decimal`; repeating scale inside every stored row would create a second source of truth.

Decimal is a dedicated catalog row rather than a `PrimitiveArray<Decimal>` alias. Static numeric
families can use metadata-free `ArrayBuilder::with_capacity`; Decimal uses
`DecimalArrayBuilder::try_with_type`. Decimal arithmetic, comparisons, casts, rounding, and
implicit coercion remain outside this chapter.

## Required and extension work

All table rows and Decimal storage checks are required. Decimal arithmetic and casts are not.
Extending the catalog with another physical family is useful practice, but it must bring the
complete scalar, array, erasure, and mismatch surface rather than one enum variant.

The starter source itself names every later target through Day 13. Future declarations and
signatures are commented at their implementation locations until their checkpoint asks you to
uncomment them. Day 5 includes numeric comparisons, Day 7 introduces three-valued Boolean logic,
and the former Days 7–12 shift to Days 8–13.

```console
cargo test -p type-exercise-starter chapter_2 --locked
cargo test -p type-exercise-starter --lib --locked
```

Next: [Chapter 3 reads several nullable column encodings](./chapter-3-column-views.md).

{{#include copyright.md}}
