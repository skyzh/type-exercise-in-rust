# Chapter 11: Build a One-Level List Column

{{#include wip-banner.md}}

List is not another primitive enum row. Each outer row points through offsets into one child array,
and the outer row can be null independently of any child value.

**Prerequisites:** Chapters 2–3, checked erased arrays, and slice ranges.

**By the end of this chapter, you will:**

- store one-level List values with explicit child type, offsets, and outer validity;
- distinguish a null list, an empty list, and a list containing a null child; and
- expose List arrays, constants, dictionaries, and typed nulls through checked views.

```console
cargo x copy-test --chapter 11
cargo test -p type-exercise-starter chapter_11 --locked
```

The first run should fail on the missing List types, invariants, and column integration.

## Keep the two null layers independent

For `n` outer rows:

```text
validity.len() == n
offsets.len() == n + 1
offsets[0] == 0
offsets are monotone
offsets[n] == child.len()
```

A null outer row and an empty non-null row both repeat an offset. Their validity bits differ. A
non-null row may span child values that include their own nulls.

## Checkpoint 1: add typed List scalars

- **Target:** `type-exercise-starter/src/array/list_array.rs::{ListScalar, ListScalarRef,
  ListError}` and List variants in
  `type-exercise-starter/src/{data_type,physical_type,scalar}.rs`.
- **Change:** retain the child physical type even for empty and all-null values; make `get`, `slice`,
  and owned conversion checked.
- **Preserve:** nested List child types return `ListError::NestedList`.
- **Run:** the Chapter 11 focused test.
- **Passing means:** borrowed and owned List values cannot lose or invent child types.

## Checkpoint 2: construct valid outer arrays

- **Target:** `type-exercise-starter/src/array/list_array.rs::{ListArray, ListArrayBuilder, try_from_rows,
  try_from_raw_parts}`.
- **Change:** validate child family, offsets, validity, and null spans before returning an array.
- **Preserve:** `ListArray::len()` is the outer row count, never the flattened child length; a
  failed row does not expose partial output.
- **Run:** focused and cumulative tests.
- **Passing means:** zero-row, all-null, empty-row, and mixed arrays retain exact invariants.

## Checkpoint 3: integrate Column views

- **Target:** `type-exercise-starter/src/column.rs::{ListColumnView, ColumnViewImpl::try_as_list}` and List erasure in
  `type-exercise-starter/src/array.rs`.
- **Change:** support List array, constant, Indexed, and typed-null representations.
- **Preserve:** Indexed validation and expected/actual type errors from Chapter 3.
- **Run:** the full Chapter 11 contract.
- **Passing means:** one-level List values reuse the existing representation boundary.

## Required and extension work

One-level storage and List inputs are required. Nested Lists, List equality as a scalar builtin,
list-producing functions, and arbitrary List casts are extensions. The public type descriptor can
represent a nested shape, but construction must reject it until those contracts exist.

```console
cargo test -p type-exercise-starter chapter_11 --locked
cargo test -p type-exercise-starter --lib --locked
```

Next: [Chapter 12 strengthens Rust type boundaries](./chapter-12-rust-boundaries.md).

{{#include copyright.md}}
