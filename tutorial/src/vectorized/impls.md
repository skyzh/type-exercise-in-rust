# Erase Physical Types at the Boundary

The database reads schemas, plans, and storage metadata at runtime. Even though a kernel wants
`&I32Array`, the executor often holds “some supported array.” The generic `Array` trait cannot be a
trait object because `get` has a different return type for each implementation.

The course uses exhaustive enums:

```rust
pub enum ArrayImpl {
    Int16(I16Array),
    Int32(I32Array),
    Int64(I64Array),
    Float32(F32Array),
    Float64(F64Array),
    Bool(BoolArray),
    String(StringArray),
    Decimal(DecimalArray),
    List(ListArray),
}

pub enum ScalarRefImpl<'a> {
    Int16(i16),
    Int32(i32),
    // ...
    String(&'a str),
    List(ListRef<'a>),
}
```

`ArrayImpl::get` returns `Option<ScalarRefImpl<'_>>`, giving type-erased diagnostic and generic
runtime code a common interface.

## Checked Downcasts

Generated expressions recover concrete references through `TryFrom`:

```rust
impl<'a> TryFrom<&'a ArrayImpl> for &'a I32Array {
    type Error = TypeMismatch;

    fn try_from(array: &'a ArrayImpl) -> Result<Self, Self::Error> {
        match array {
            ArrayImpl::Int32(array) => Ok(array),
            other => Err(TypeMismatch(
                PhysicalType::Int32,
                other.physical_type(),
            )),
        }
    }
}
```

Function authors do not call this conversion. Binding selects the concrete generic expression, and
`ColumnView::try_from` uses the shared conversion once per batch.

## Why an Enum?

An exhaustive enum is appropriate when the framework owns a closed set of physical layouts. It
provides:

- exhaustive compiler checks when a physical type is added;
- direct matching without vtables;
- one place to implement diagnostics and conversions; and
- a natural `PhysicalType` mapping.

The repository also contains `BoxedArray`, built from an object-safe `DynArray` companion trait, to
support nested lists. This illustrates the alternative: a trait object can expose a smaller erased
API and downcast through `Any`. Neither strategy makes the original generic `Array` object safe.

Use an enum for a closed runtime universe and a trait object when third-party or recursively nested
implementations must participate without editing the enum.

## Generate Repetitive Implementations

Hand-writing `From`, `TryFrom`, `get`, `len`, builder dispatch, and physical-type matches for every
variant would make the list easy to desynchronize. `for_all_variants!` stores the association once:

```rust
{ Int32, int32, I32Array, I32ArrayBuilder, i32, i32 }
{ String, string, StringArray, StringArrayBuilder, String, &'a str }
```

Callback macros consume this list to generate each repetitive implementation. The next chapters use
the same pattern for expression families, but keep the purpose distinct:

- the physical-type list prevents boilerplate drift;
- the numeric combination list records SQL promotion policy.

They should not be merged merely because both are macros.

## Chapter Checkpoint

Trace an `ArrayImpl::String` input through `ColumnViewImpl`, `ColumnView<String>`, and
`ArrayColumnView<String>` until a kernel receives `&str`. Identify the only runtime match at each
layer.

Next, [generate vectorizers](./func.md) around ordinary scalar functions.

{{#include ../copyright.md}}
