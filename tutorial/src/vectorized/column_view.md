# Column Views: Arrays, Constants, and Dictionaries

`ColumnView<'a, S>` is the main new abstraction in the revised course. It represents a logical
nullable column of scalar type `S` without requiring one physical layout.

```rust
pub enum ColumnView<'a, S: Scalar> {
    Array(ArrayColumnView<'a, S>),
    Constant(ConstantColumnView<'a, S>),
    Dictionary(DictionaryColumnView<'a, S>),
}
```

Every variant implements the static hot-loop interface:

```rust
pub trait ColumnAccessor<'a, S: Scalar> {
    fn len(&self) -> usize;
    fn get(&self, row: usize) -> Option<S::RefType<'a>>;
}
```

## Regular Array View

`ArrayColumnView<'a, S>` borrows `S::ArrayType`. Its `get` delegates to the Arrow-like array. A
production integration could implement another accessor around an `arrow-rs` array as long as it
returns the same `S::RefType<'a>` and null semantics.

The course does not add Apache Arrow as a dependency merely to rename the existing arrays. The
interface boundary is the important part: storage adapters should not change scalar kernels.

## Constant View

A constant view stores one optional borrowed scalar and a logical length:

```rust
ConstantColumnView {
    value: Some(42),
    len: 65_536,
}
```

It allocates no 65,536-element input array. A typed null constant also carries `PhysicalType`, so
binding remains checkable even though the value is `None`.

## Dictionary View

A dictionary view contains nullable indices and one regular values array:

```rust
DictionaryColumnView {
    indices: &[Some(1), None, Some(0)],
    values: &["red", "green"],
}
```

The logical values are `green`, null, and `red`. `ColumnViewImpl::dictionary` validates every
non-null key once at construction. The hot loop can then index the dictionary without repeating a
structural validation.

## Typed and Type-Erased Forms

The runtime boundary uses `ColumnViewImpl<'a>`:

```rust
pub enum ColumnViewImpl<'a> {
    Array(&'a ArrayImpl),
    Constant { /* ScalarRefImpl, PhysicalType, len */ },
    Dictionary { /* indices, values: &ArrayImpl */ },
}
```

`TryFrom<ColumnViewImpl<'a>> for ColumnView<'a, S>` performs the physical type check and converts the
erased scalar or array reference. This happens once per input per batch.

## Dispatch Once, Not Once per Row

A first implementation called `ColumnView::get`, which matched the enum for every row. The
benchmark showed the generated array/array kernel was roughly 45% slower than a hand-written loop.

The generator now matches view variants before entering the loop and calls a generic
`eval_typed<V1, V2, ...>` method. `V1` and `V2` are concrete accessor structs, so the compiler
monomorphizes `get`. After this change, the regular generated path is within a few percent of the
hand-written baseline on the development machine.

The lesson is broader than this implementation: type erasure is inexpensive when it occurs around a
large batch; representation dispatch inside the tight row loop may not be.

## Task

Add a fourth view on paper: a selection view that maps logical row `r` through a selection vector
before reading another accessor. Decide:

- whether it should be validated at construction;
- whether it owns or borrows the inner accessor;
- what lifetime its `get` returns; and
- how generated one-time dispatch would handle nested views without an exponential enum.

Next, erase [physical types at the framework boundary](./impls.md).

{{#include ../copyright.md}}
