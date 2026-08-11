{{#include wip-banner.md}}

# Chapter 3: Read Nullable Columns Without Materializing Them

A scalar kernel should not care whether a logical row comes from an array, one repeated constant,
or an index into shared values. This chapter gives those representations one checked borrowed
interface.

**Prerequisites:** Chapters 1–2, slices, and validity-based nulls.

**By the end of this chapter, you will:**

- read array, constant, Indexed, and typed-null columns through `ColumnViewImpl`;
- preserve null rows without materializing another array; and
- reject every invalid index before a view becomes usable.

Open the existing Day 3 skeleton in `type-exercise-starter/src/column.rs`. Follow its checkpoint
comments, uncomment the declarations you are implementing, then uncomment the matching module and
public export in `type-exercise-starter/src/lib.rs`:

```rust,ignore
mod column;
pub use column::{ColumnView, ColumnViewImpl};
```

Keep the public views in `column.rs`; this module wiring lets the copied test import them from the
starter crate root.

```console
cargo x copy-test --chapter 3
cargo test -p type-exercise-starter chapter_3 --locked
```

The first run should fail on the still-commented column constructors and typed view. After you
uncomment their skeletons, it should reach your missing implementation rather than any tool-driven
source rewrite.

## Representation and logical rows

These columns describe the same logical values:

```text
array:    [10, 20, null]
constant: 10 repeated three times
Indexed:  indices [0, 1, null] over values [10, 20]
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

- **Target:** `type-exercise-starter/src/column.rs::ColumnViewImpl::{array, constant, null, indexed}`.
- **Change:** borrow the backing values and record one logical row count.
- **Preserve:** constructors do not copy array values; typed nulls retain type and length.
- **Run:** the Chapter 3 focused test.
- **Passing means:** all four representations expose the expected row count and physical type.

Validate every non-null index in the constructor and return `Err` before exposing a usable view; do
not wait for a later `get` to panic. Choose a readable error representation, but the supplied tests
do not require a particular public error type, field layout, or display text.

## Checkpoint 2: recover one typed view

- **Target:** `type-exercise-starter/src/column.rs::{ColumnView, ColumnView::get, ColumnView::len}` and
  `TryFrom<ColumnViewImpl>`.
- **Change:** check the family once and read each representation as nullable logical rows.
- **Preserve:** null indices and null values both become `None`.
- **Run:** the focused and cumulative tests.
- **Passing means:** expanded primitive families work through array, constant, and Indexed
  views without family-specific row loops.

## Required and extension work

The four representations and fail-closed Indexed constructor are required. Run-length encoding
and nested columns are extensions. List will reuse the same representation boundary in Chapter 10.

```console
cargo test -p type-exercise-starter chapter_3 --locked
cargo test -p type-exercise-starter --lib --locked
```

Before continuing, explain why an all-null column still needs a physical type and why index
validation belongs in construction rather than row evaluation.

## Test your understanding

Compare this borrowed Indexed view with Arrow's
[`DictionaryArray<K>`](https://arrow.apache.org/rust/arrow/array/struct.DictionaryArray.html) and
DataFusion's [`ArrayRef`](https://datafusion.apache.org/user-guide/arrow-introduction.html) /
[`ColumnarValue`](https://docs.rs/datafusion/latest/datafusion/logical_expr/enum.ColumnarValue.html)
boundary. Which representation owns nullable primitive keys plus shared values, validates key
bounds, and remains an array with a key-type parameter, slicing, and builders? Why can this
course's borrowed indices plus ordinary `ArrayImpl` teach checked indirection without becoming a
persistable, interchangeable dictionary encoding?

Next: [Chapter 4 exposes what unary and binary loops repeat](./chapter-4-concrete-loops.md).

{{#include copyright.md}}
