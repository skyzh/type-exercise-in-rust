{{#include wip-banner.md}}

# Chapter 3: Read Nullable Columns Without Materializing Them

Chapter 2 gave the executor several physical families. A row loop still should not need separate
code for an array, one scalar repeated across a batch, or an index into shared values. Those are
different representations of a column, not different scalar operations.

This chapter gives them one borrowed boundary. `ColumnViewImpl<'a>` keeps the representation known
at runtime. After one checked conversion, `ColumnView<'a, S>` lets generic code read nullable rows
as `Option<S::RefType<'a>>`. The views borrow their buffers, so constants and indexed columns do not
need to materialize another array first.

## What is in the starter

Begin from your completed Chapter 2 workspace. The scalar, array, erasure, logical-type, and
Decimal work from the first two chapters is already present. `src/column.rs` contains only the Day
3 comment shells for `ColumnViewImpl<'a>` and `ColumnView<'a, S>`. The `column` module and its public
exports remain commented in `src/lib.rs`.

You own two additions in this chapter:

1. a representation-erased borrowed view with checked constructors; and
2. a typed borrowed view that checks one physical family before row access.

Later relationships in the starter remain comments. Do not implement Day 7 nullability proofs or
Day 12 List views here.

Copy the cumulative supplied test before editing:

```console
cargo x copy-test --chapter 3
cargo test -p type-exercise-starter-supplied-tests chapter_3 --locked
```

The first run should fail because `ColumnViewImpl` and `ColumnView` are not exported yet. Do not
edit the copied test. The two checkpoints below use one final Chapter 3 contract, so Checkpoint 1
has a library compile gate and Checkpoint 2 makes the focused test green.

## Checkpoint 1: Borrow each physical representation

Open `src/column.rs` and implement `ColumnViewImpl<'a>`. It represents four ways a batch can supply
logical rows:

| Representation | Borrowed state | Logical length | Physical type |
| --- | --- | --- | --- |
| Array | `&'a ArrayImpl` | array length | array physical type |
| Constant | one `ScalarRefImpl<'a>` plus a length | recorded length | scalar physical type |
| Typed null | `PhysicalType` plus a length | recorded length | recorded physical type |
| Indexed | compact `&'a [u32]` keys plus `&'a ArrayImpl` values | key count | values physical type |

The lifetime `'a` is the ownership boundary: the view may borrow an array, keys, or a string
scalar, but it does not own or copy those buffers. Implement `array`, `constant`, `null`, and
`indexed`, together with `len`, `is_empty`, `physical_type`, and erased row access with this exact
shape:

```rust,ignore
pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'a>>
```

Keep the representation enum private behind the public `ColumnViewImpl` wrapper. This small split
forces callers through the constructors, so they cannot bypass the indexed bounds check. It also
leaves one place for later chapters to attach batch-wide metadata instead of repeating that state
inside every representation variant.

A typed null needs an explicit `PhysicalType` because it has no non-null scalar from which to
recover one. The type still matters for overload selection and output allocation, including for an
empty batch. Do not add nullable variants to `DataType`, `PhysicalType`, or the scalar families;
this chapter continues to represent each logical row as `Some(value)` or `None`.

For an indexed view, each compact non-null `u32` key selects one row from the borrowed values
array. A null logical row lives in that nullable values array rather than in the key buffer:

```text
keys[row] = i, values[i] null -> None
keys[row] = i, values[i] set  -> Some(values[i])
```

`array`, `constant`, and `null` are direct constructors. `indexed` is the fallible constructor:
validate every key inside it before returning a view. If any key is outside the values array,
return an ordinary `anyhow::Error` that identifies the row, key, and values length, and expose no
partially valid view.

Enable the module for this checkpoint and export only the type you have implemented:

```rust,ignore
mod column;
pub use column::ColumnViewImpl;
```

Then compile the learner library:

```console
cargo check -p type-exercise-starter-expr --lib --locked
```

Passing means the real `column.rs` implementation compiles. The focused Chapter 3 test is still
expected to fail because `ColumnView<'a, S>` belongs to Checkpoint 2.

## Checkpoint 2: Check the scalar family once

Now implement `ColumnView<'a, S>` and `TryFrom<ColumnViewImpl<'a>>`. The erased view can report its
`PhysicalType`, but a generic row loop wants the concrete family `S`. Compare the view's physical
type with `S::PHYSICAL_TYPE` once during conversion. A mismatch returns `TypeMismatch` before any
row is read.

After that check, recover the matching borrowed array, borrowed scalar reference, or indexed values
array once and store it in the typed view. `get(row)` can then return `Option<S::RefType<'a>>`
without repeating an erased downcast for every row. The GAT relationship from Chapter 1 remains
visible: `ColumnView<'a, i32>` returns copied `i32` values, while `ColumnView<'a, String>` returns
`&'a str` borrowed from the original string storage.

The generic typed view covers the catalog families that implement `Scalar`. `ColumnViewImpl` can
still carry an erased Decimal array and preserve its exact `PhysicalType::Decimal` descriptor.
Decimal does not implement the static `Scalar` relationship, so it is not a `ColumnView<Decimal>`
family in this chapter.

Finish the public export in `src/lib.rs`:

```rust,ignore
pub use column::{ColumnView, ColumnViewImpl};
```

Run the focused contract, then all learner-library tests copied so far:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_3 --locked
cargo test -p type-exercise-starter-expr --lib --locked
```

The focused test proves five boundaries:

- arrays, constants, and indexed values expose the same logical-row interface;
- the primitive families added in Chapter 2 work without family-specific row loops;
- typed-null and empty views retain their type and length;
- every invalid key is rejected during construction; and
- a physical-family mismatch fails before row access.

Keep this chapter focused on borrowed execution views. The indexed form borrows compact `u32` keys
and an existing nullable `ArrayImpl`; it is not a persisted dictionary-array format and adds no
key-array family, builder, or storage encoding. Leave run-length encoding as an extension. Chapter
7 adds primitive-loop specialization, and Chapter 12 reuses this representation boundary for
List.

Before continuing, make sure you can explain three boundaries in your own words:

1. Why must an all-null or empty column carry a physical type instead of inferring one from rows?
2. Why does `indexed` validate every key before it returns a view?
3. Why can `ColumnView<'a, String>::get` return a borrowed `&'a str` without materializing a new
   `StringArray`?

You can now separate the representation of a batch from the scalar operation that reads it.
Chapter 4 will use this borrowed boundary to expose what concrete unary and binary row loops repeat.

Next: [Chapter 4 exposes what unary and binary loops repeat](./chapter-4-concrete-loops.md).

{{#include copyright.md}}
