# Checkpoint 1: Build Physical Types and Arrays

Build the values that every later expression will read and write. By the end of this checkpoint,
the starter will know eight physical families and will store nullable fixed-width, string, and
Decimal values in dense arrays.

Start by copying the public checkpoint test into your starter and running it:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter-supplied-tests chapter_1 --locked
```

The copied test should not compile yet. Its missing names and trait implementations are your work
list. Do not edit the copied test; work in `type-exercise-starter/core/src` instead.

## Connect the physical families

An execution engine needs both compile-time Rust types and runtime tags. Define these eight rows in
`physical_type.rs` and `variant_catalog.rs`, keeping this order:

| Physical type | Owned scalar | Borrowed scalar | Dense array |
| --- | --- | --- | --- |
| `Int16` | `i16` | `i16` | `I16Array` |
| `Int32` | `i32` | `i32` | `I32Array` |
| `Int64` | `i64` | `i64` | `I64Array` |
| `Bool` | `bool` | `bool` | `BoolArray` |
| `Float32` | `f32` | `f32` | `F32Array` |
| `Float64` | `f64` | `f64` | `F64Array` |
| `String` | `String` | `&str` | `StringArray` |
| `Decimal` | `Decimal` | `Decimal` | `DecimalArray` |

`PhysicalType` carries runtime information. Most variants are simple tags, while
`Decimal(DecimalType)` also carries precision and scale. `PhysicalFamily` is descriptor-free and
exists so `PHYSICAL_FAMILY_CATALOG` can audit the eight supported rows.

In `scalar.rs`, complete the reciprocal relationships among `Scalar`, `ScalarRef`, and `Array`.
The generic relationship should be strong enough that code with only `S: Scalar` can discover
`S::RefType<'a>` and `S::ArrayType`, and an array can point back to the same scalar family.

The generic associated type matters for strings: an integer read is copied, but an `&'a str` must
stay tied to the array that owns its bytes.

```rust,ignore
fn first_value<S: Scalar>(array: &S::ArrayType) -> Option<S> {
    array.get(0).map(ScalarRef::to_owned_scalar)
}
```

Use the catalog callback to generate the repeated scalar and array connections. Implement the
erased `ScalarImpl`, `ScalarRefImpl`, and `ArrayImpl` boundaries too. Upcasts through `From` cannot
fail; downcasts through `TryFrom` must report the actual and expected physical types instead of
panicking or reinterpreting bytes.

## Map logical types to storage

Define `DecimalType` and `Decimal` in `decimal.rs`. Check precision and scale when the descriptor
is created, and reject an unscaled coefficient that cannot fit its precision. A Decimal array has
one descriptor shared by every row; it does not repeat precision and scale beside each `i128`.

Define the planner-facing `DataType` in `data_type.rs`. Map SQL names such as `SmallInt`,
`Integer`, `Varchar`, and `Decimal` to the physical families above. Add the string and numeric
classifiers used by the public test.

## Store fixed-width values densely

Replace the marker types in `array/primitive_array.rs` with two buffers:

- `Vec<T>` contains one value slot per row.
- `BitVec` contains one validity bit per row; `true` means non-null.

A null row still has a value slot. Store `T::default()` there and treat it as ignored—the validity
bit is the only source of nullness. Implement the six fixed-width aliases with one generic
`PrimitiveArray<T>` and one generic builder. Expose read-only `values()` and `validity()` accessors
so callers can inspect the layout without mutating it.

## Store strings without one allocation per row

In `array/string_array.rs`, use three buffers:

- `Vec<u8>` stores all UTF-8 bytes.
- `Vec<usize>` stores `row_count + 1` nondecreasing offsets.
- `BitVec` stores row validity.

Row `i` occupies `offsets[i]..offsets[i + 1]`. Null and empty strings may repeat an offset; the
validity bit distinguishes them. `get` should return an `&str` borrowed directly from the byte
buffer.

Do not add a transactional string writer, slicing API, or column view here. Those capabilities are
owned by later checkpoints.

## Keep Decimal metadata stable

In `array/decimal_array.rs`, wrap dense `i128` storage with one `DecimalType`. Validate raw-part
lengths and every non-null coefficient. A builder must reject a row with different Decimal
metadata before changing its length or buffers.

## Run the checkpoint

Run the same learner command until all five public behaviors pass:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter-supplied-tests chapter_1 --locked
```

You can compare against the completed checkpoint without changing the starter:

```console
cargo test -p type-exercise-checkpoint-01-supplied-tests --locked
cargo check -p type-exercise-checkpoint-01-core --locked
```

Checkpoint 1 is complete when the catalog has all eight rows, dense arrays preserve values and
null positions, string reads borrow from the shared bytes, Decimal builders reject incompatible
metadata without mutation, and erased downcasts fail safely for the wrong family.

The next checkpoint will add nullable `Array`, `Constant`, and `Indexed` column views. It will use
these arrays rather than replacing them.

{{#include copyright.md}}
