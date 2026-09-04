mod primitive_array;
mod string_array;

// Chapter 1: uncomment after implementing Decimal metadata and storage.
// mod decimal_array;
// pub use decimal_array::{DecimalArray, DecimalArrayBuilder};

// Chapter 9: uncomment this module and its public surface after implementing
// `array/list_array.rs`; keep `ListArrayBuilder` private to the module.
// mod list_array;
// pub use list_array::{ListArray, ListError, ListScalar, ListScalarRef};
// Chapter 10 inspection: `Array::iter` is already opaque and tied to the array borrow. Its concrete
// implementation is not a learner-owned API or checkpoint task.

pub use primitive_array::{I32Array, I32ArrayBuilder, PrimitiveArray, PrimitiveArrayBuilder};
// Chapter 1: extend the primitive re-export with I16, I64, Bool, F32, and F64 arrays
// and builders.
// Chapter 6: keep this single primitive representation; the private raw binding borrows
// its values and validity through `ColumnViewImpl`.
pub use string_array::{StringArray, StringArrayBuilder};
// Chapter 4: extend this re-export with `Writer` and `WriterUsed`.

/// Chapter 1: add the bounds that connect an array to its scalar and builder forms.
pub trait Array {
    type Builder;
    type OwnedItem;
    type RefItem<'a>;
    fn get(&self, row: usize) -> Option<Self::RefItem<'_>>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn iter<'a>(&'a self) -> impl Iterator<Item = Option<Self::RefItem<'a>>> + 'a {
        (0..self.len()).map(|row| self.get(row))
    }
    fn from_slice(items: &[Option<Self::RefItem<'_>>]) -> Self;
}

/// Chapter 1: add the reciprocal bounds and item relationship for append-only builders.
pub trait ArrayBuilder {
    type Array;
    type RefItem<'a>;
    fn with_capacity(capacity: usize) -> Self;
    fn push(&mut self, value: Option<Self::RefItem<'_>>);
    fn finish(self) -> Self::Array;
}

/// The two erased array variants visible at the start of Chapter 1.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrayImpl {
    Int32(I32Array),
    String(StringArray),
    // Chapter 1: add the remaining primitive and Decimal variants.
    // Chapter 9: add `List(ListArray)`.
}

// Chapter 1: add erased dispatch for Int32 and String plus their explicit owned `From`,
// owned `TryFrom`, and borrowed `TryFrom` conversions. Keep these two families handwritten;
// Chapter 1 introduces catalog-driven generation when the physical-family inventory scales.
