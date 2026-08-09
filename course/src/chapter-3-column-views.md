# Chapter 3: Read Nullable Columns Without Materializing Them

A scalar kernel should not care whether a logical row comes from an array, one repeated constant,
or a dictionary key. This chapter gives those representations one checked borrowed interface.

**Prerequisites:** Chapters 1–2, slices, and validity-based nulls.

**By the end of this chapter, you will:**

- read array, constant, dictionary, and typed-null columns through `ColumnViewImpl`;
- preserve null rows without materializing another array; and
- reject every invalid dictionary key before a view becomes usable.

```console
cargo x copy-test --chapter 3
cargo test -p type-exercise-starter chapter_3 --locked
```

The first run should fail on the missing column constructors and typed view.

## Representation and logical rows

These columns describe the same logical values:

```text
array:      [10, 20, null]
constant:   10 repeated three times
dictionary: keys [0, 1, null] over values [10, 20]
```

`ColumnViewImpl<'a>` stores the representation. `ColumnView<'a, S>` proves the physical family
once, then `get(row)` returns `Option<S::RefType<'a>>`. The future expression loop sees only rows.

## Infer a Type When Every Row Is Null

A non-null constant carries its physical type in its value. An all-null column does not. The
planner must still choose an overload and allocate an output array—even for an empty batch—so a
typed-null view carries `PhysicalType` beside its length.

Nullability remains row state. Do not add `DataType::Nullable`, `PhysicalType::Nullable`, or a
separate nullable scalar family.

## Checkpoint 1: construct checked representations

- **Target:** `type-exercise-starter/src/column.rs::ColumnViewImpl::{array, constant, null, dictionary}`.
- **Change:** borrow the backing values and record one logical row count.
- **Preserve:** constructors do not copy array values; typed nulls retain type and length.
- **Run:** the Chapter 3 focused test.
- **Passing means:** all four representations expose the expected row count and physical type.

Validate every non-null dictionary key in the constructor. Report its logical row, key, and values
length in `InvalidDictionaryKey`; do not wait for a later `get` to panic.

## Checkpoint 2: recover one typed view

- **Target:** `type-exercise-starter/src/column.rs::{ColumnView, ColumnView::get, ColumnView::len}` and
  `TryFrom<ColumnViewImpl>`.
- **Change:** check the family once and read each representation as nullable logical rows.
- **Preserve:** dictionary null keys and null values both become `None`.
- **Run:** the focused and cumulative tests.
- **Passing means:** expanded primitive families work through array, constant, and dictionary
  views without family-specific row loops.

## Required and extension work

The four representations and fail-closed dictionary constructor are required. Run-length encoding
and nested columns are extensions. List will reuse the same representation boundary in Chapter 10.

```console
cargo test -p type-exercise-starter chapter_3 --locked
cargo test -p type-exercise-starter --lib --locked
```

Before continuing, explain why an all-null column still needs a physical type and why dictionary
validation belongs in construction rather than row evaluation.


{{#include copyright.md}}
