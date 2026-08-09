# Chapter 2: Scale the Physical Type Family

Adding `f64` beside the two Chapter 1 rows repeats physical variants, scalar variants, array
aliases, builders, and conversions. Repeating that work for every primitive makes drift more likely
than the type itself warrants.

**Prerequisites:** Chapter 1 and basic declarative macros.

**By the end of this chapter, you will:**

- add complete `i16`, `i64`, `bool`, `f32`, `f64`, and Decimal storage families;
- map planner-visible `DataType` values to physical storage; and
- keep the non-List physical rows in one catalog that drives exhaustive code.

```console
cargo x copy-test --chapter 2
cargo test -p type-exercise-starter chapter_2 --locked
```

The first run should fail on the new families and catalog.

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
| `Decimal` | `Decimal` | `Decimal` / `Decimal` |

`Char { width }` and `Decimal { scale, precision }` retain logical metadata, but this course does
not validate width, precision, or scale in physical storage.

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

## Required and extension work

All table rows are required. Decimal arithmetic and precision enforcement are not. Extending the
catalog with another physical family is useful practice, but it must bring the complete scalar,
array, erasure, and mismatch surface rather than one enum variant.

```console
cargo test -p type-exercise-starter chapter_2 --locked
cargo test -p type-exercise-starter --lib --locked
```

Next: [Chapter 3 reads several nullable column encodings](./chapter-3-column-views.md).

{{#include copyright.md}}
