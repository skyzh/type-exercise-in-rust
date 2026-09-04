{{#include wip-banner.md}}

# Checkpoint 2: Add Nullable Column Views

Checkpoint 1 gave you owned arrays. Now borrow those arrays in three useful shapes without copying
their values:

- an `Array` view reads rows in their original order;
- a `Constant` repeats one value or typed null for a requested length; and
- an `Indexed` view remaps rows through a borrowed index slice.

Begin from your completed Checkpoint 1 workspace. Copy the cumulative tests, then run only the new
Chapter 2 cases once:

```console
cargo x copy-test --chapter 2
cargo test -p type-exercise-starter-supplied-tests chapter_2 --locked
```

That focused test should fail because `ColumnViewImpl` and `ColumnView` do not exist yet. The
Chapter 1 implementation should still compile. Do not edit the copied tests.

## Enable the learner-owned module

Open `type-exercise-starter/core/src/lib.rs` and enable the existing `column` module and export.
Then implement `type-exercise-starter/core/src/column.rs`.

The erased view must accept every Checkpoint 1 physical family. Keep its representation private
and expose checked constructors instead:

```rust,ignore
let values: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
let array = ColumnViewImpl::array(&values);

let constant = ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 3);
let nulls = ColumnViewImpl::null(PhysicalType::Int32, 3);

let indices = [2, 1, 2, 0];
let indexed = ColumnViewImpl::indexed(&indices, &values)?;
```

All three forms answer the same questions: `len`, `is_empty`, `physical_type`, and `get`. They
borrow their inputs for lifetime `'a`; none of the constructors should allocate an output array.

## Preserve nulls and physical types

An array view delegates its length, physical type, and row read to the borrowed `ArrayImpl`. A
constant stores one `Option<ScalarRefImpl<'a>>` and a length:

- `constant(value, len)` records `value.physical_type()` and returns that same value for every row;
- `null(physical_type, len)` records the supplied type and returns `None` for every row.

The explicit type on a null constant is essential. `None` carries no scalar variant, but later
code must still distinguish a null Int64 column from a null String column.

Treat `row < len` as the public precondition for `get`, matching array access in this course.
Assert that bound before reading the private representation. Inside a valid range, `None` means a
SQL null rather than an out-of-bounds sentinel.

## Validate indexed views once

An indexed view borrows `&[u32]` and an `&ArrayImpl`. Its output length is the number of indices,
and its physical type is the values array's type. Output row `r` reads
`values.get(indices[r] as usize)`.

Validate every index in `ColumnViewImpl::indexed`. If an index is outside the values array, return
an error that identifies the bad index and its output row. A successfully constructed view can
then read every output row without repeating bounds validation or creating a gathered array.

For example, values `["zero", NULL, "two"]` with indices `[2, 1, 2, 0]` read as
`["two", NULL, "two", "zero"]`. The two appearances of `"two"` borrow the same underlying
string bytes.

## Check the scalar family once

`ColumnViewImpl` is appropriate when a planner knows the physical type only at runtime. Generic
code often wants a concrete scalar family. Add `ColumnView<'a, S: Scalar>` with the same three
private forms and implement:

```rust,ignore
TryFrom<ColumnViewImpl<'a>> for ColumnView<'a, S>
```

Compare the erased view's `physical_type()` with `S::PHYSICAL_TYPE` before converting its private
state. Then downcast the borrowed array, constant scalar, or indexed values array through the
checked conversions from Checkpoint 1. A mismatched family returns `TypeMismatch`.

After that one conversion, `ColumnView<'a, S>::get` returns `Option<S::RefType<'a>>` directly. For
`ColumnView<'_, String>`, the returned `&str` still borrows the original `StringArray` bytes.
Decimal remains available through `ColumnViewImpl`; its precision and scale are runtime metadata,
so it does not use the static `Scalar` relationship.

## Run both checkpoints

Run the focused Chapter 2 cases, then the full cumulative package:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_2 --locked
cargo test -p type-exercise-starter-supplied-tests --locked
```

The cumulative run should pass nine tests: five from Checkpoint 1 and four from Checkpoint 2. You
can run the completed snapshot independently:

```console
cargo test -p type-exercise-checkpoint-02-supplied-tests --locked
cargo check -p type-exercise-checkpoint-02-core --locked
```

Checkpoint 2 is complete when array views preserve null positions, constants repeat values and
typed nulls, indexed views validate and remap rows, and typed views reject the wrong family before
returning borrowed scalar references.

The next checkpoint will use these views for shared expression evaluation and numeric
instantiation. Do not add that execution layer yet.

{{#include copyright.md}}
