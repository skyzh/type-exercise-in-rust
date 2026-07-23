# Owned and Borrowed Scalars

Array builders accept borrowed values, but scalar functions often produce owned values. String
concatenation, for example, consumes two `&str` values and returns a `String`. The framework needs a
generic path from that owned result back to the output builder.

`Scalar` and `ScalarRef` express the relationship:

```rust
pub trait Scalar: Clone + Send + Sync + 'static {
    const PHYSICAL_TYPE: PhysicalType;
    type ArrayType: Array<OwnedItem = Self>;
    type RefType<'a>: ScalarRef<
        'a,
        ScalarType = Self,
        ArrayType = Self::ArrayType,
    >;

    fn as_scalar_ref(&self) -> Self::RefType<'_>;
}

pub trait ScalarRef<'a>: Copy + Send + 'a {
    type ArrayType: Array<RefItem<'a> = Self>;
    type ScalarType: Scalar<RefType<'a> = Self>;

    fn to_owned_scalar(&self) -> Self::ScalarType;
}
```

For primitives, owned and borrowed forms are the same:

```text
i32 <-> i32 <-> PrimitiveArray<i32>
```

For strings, they differ:

```text
String <-> &'a str <-> StringArray
```

Lists use another borrowed view:

```text
List <-> ListRef<'a> <-> ListArray
```

## Equality Constraints Are the Point

The associated types are not just aliases. Their bounds prove facts used by the vectorizer:

```text
S::ArrayType::OwnedItem == S
S::ArrayType::RefItem<'a> == S::RefType<'a>
S::RefType<'a>::ScalarType == S
```

When `func(left, right)` returns `O`, the loop can call `as_scalar_ref()` and pass the result to
`O::ArrayType::Builder` without a runtime conversion.

## Higher-Ranked Bounds

Some relationships must hold for every possible borrow lifetime. Rust writes that requirement with
a higher-ranked trait bound (HRTB):

```rust
where
    for<'a> Self::ArrayType: Array<RefItem<'a> = Self::RefType<'a>>
```

Comparison kernels may receive two borrowed values with different lifetimes. Their real requirement
is also stated directly:

```rust
for<'a, 'b> C::RefType<'a>: PartialOrd<C::RefType<'b>>
```

The current implementation does not carry the original course's manual GAT lifetime-upcast method.
The direct cross-lifetime bound describes the operation more accurately and compiles on the pinned
stable toolchain.

## Physical Type Identity

`Scalar::PHYSICAL_TYPE` lets `ColumnView<String>` reject an `Int32` input before entering the loop.
The constant view also needs this identity when its value is null: `None` alone does not reveal
whether it is a null integer or null string.

## Task

Trace `String` through these operations:

1. `StringArray::get` returns `&str`;
2. `str_contains` consumes two `&str` values and returns `bool`;
3. `bool::as_scalar_ref` returns `bool`; and
4. `BoolArrayBuilder::push` stores it with a validity bit.

Then repeat the trace for a function that returns an owned `String`. Identify exactly how long the
owned temporary must remain alive before the builder copies its bytes.

Next, make kernels independent of the [column's physical encoding](./column_view.md).

{{#include ../copyright.md}}
