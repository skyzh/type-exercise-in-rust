# Arrow-like Arrays and Builders

An array stores nullable values of one physical type. The course implements two important layouts:

- `PrimitiveArray<T>` stores a dense value buffer and a separate validity bitmap;
- `StringArray` stores flat UTF-8 bytes, offsets, and a validity bitmap.

For `[Some("db"), None, Some("rust")]`, a string array resembles:

```text
data:    dbrust
offsets: 0, 2, 2, 6
valid:   1, 0, 1
```

This layout returns `&str` without constructing a `String` per row.

## One Trait, Different Borrowed Values

The `Array` trait uses a generic associated type because its borrowed item may depend on the array
borrow's lifetime:

```rust
pub trait Array: Sized + Send + Sync + 'static {
    type Builder: ArrayBuilder<Array = Self>;
    type OwnedItem: Scalar<ArrayType = Self>;
    type RefItem<'a>: ScalarRef<
        'a,
        ScalarType = Self::OwnedItem,
        ArrayType = Self,
    >;

    fn get(&self, index: usize) -> Option<Self::RefItem<'_>>;
    fn len(&self) -> usize;
}
```

The concrete choices are natural:

```rust
impl Array for PrimitiveArray<i32> {
    type RefItem<'a> = i32;
    // ...
}

impl Array for StringArray {
    type RefItem<'a> = &'a str;
    // ...
}
```

GATs are stable Rust and require no feature flag.

## Reciprocal Builder Types

An expression knows its output array type `O`, so it must derive the correct builder and know what
`finish` returns:

```rust
pub trait ArrayBuilder: Sized {
    type Array: Array<Builder = Self>;

    fn with_capacity(capacity: usize) -> Self;
    fn push(&mut self, value: Option<<Self::Array as Array>::RefItem<'_>>);
    fn finish(self) -> Self::Array;
}
```

Both directions matter:

```text
Array::Builder::Array == Array
ArrayBuilder::Array::Builder == ArrayBuilder
```

Without those equality constraints, a generic expression cannot prove that finishing `O::Builder`
returns `O`.

## Iteration

`ArrayIterator<'a, A>` stores `&'a A` and a row position. Its item is
`Option<A::RefItem<'a>>`. One implementation now covers fixed-width primitives, variable-width
strings, decimals, booleans, and lists.

## Proving an All-Valid Primitive Batch

Primitive arrays cache `null_count` alongside their validity bitmap. This keeps ordinary nullable
`get` exactly as cheap as before while allowing one constant-time check before a numeric loop:

```rust
if let Some(non_null) = array.as_non_null() {
    let values: &[i32] = non_null.values();
    // No validity lookup is needed while reading this slice.
}
```

`NonNullPrimitiveArray<'a, T>` is a checked proof wrapper, not another logical data type or another
`ColumnView` representation. The array remains an ordinary `PrimitiveArray<T>` at framework
boundaries. `PrimitiveArray::from_values` constructs an all-valid output and initializes its bitmap
in bulk, avoiding one bitmap push per result row.

## Task

Implement or inspect these operations in `expr-common/src/array`:

1. `PrimitiveArrayBuilder::push`, including the placeholder value for null rows;
2. `StringArray::get`, using adjacent offsets;
3. `StringArrayBuilder::push`, ensuring nulls repeat the previous offset;
4. `ArrayIterator::next`; and
5. `PrimitiveArray::as_non_null`, including the cached proof it relies on.

Do not use unchecked indexing unless the surrounding API proves the row is in bounds. The existing
string implementation uses unchecked UTF-8 conversion because builders only accept `&str`; that
invariant should be documented where the unsafe operation occurs.

## Chapter Checkpoint

You should be able to write a generic function that copies any array:

```rust
fn copy_array<A: Array>(input: &A) -> A {
    let mut output = A::Builder::with_capacity(input.len());
    for value in input.iter() {
        output.push(value);
    }
    output.finish()
}
```

Next, connect arrays to [owned and borrowed scalar values](./scalar.md).

{{#include ../copyright.md}}
