mod primitive_array;
mod string_array;

// Day 2, checkpoint 4: uncomment after implementing Decimal metadata and storage.
// mod decimal_array;
// pub use decimal_array::{DecimalArray, DecimalArrayBuilder};

// Day 12, checkpoints 1–2: uncomment this module and its public surface after implementing
// `array/list_array.rs`; keep `ListArrayBuilder` private to the module.
// mod list_array;
// pub use list_array::{ListArray, ListError, ListScalar, ListScalarRef};
// Day 13, checkpoint 1: uncomment when replacing `Array::iter` with the private iterator.
// mod iterator;

pub use primitive_array::{I32Array, I32ArrayBuilder, PrimitiveArray, PrimitiveArrayBuilder};
// Day 2, checkpoint 1: extend the primitive re-export with I16, I64, Bool, F32, and F64 arrays
// and builders.
// Day 7, checkpoint 1: keep this single primitive representation; the private raw binding borrows
// its values and validity through `ColumnViewImpl`.
pub use string_array::{StringArray, StringArrayBuilder};
// Day 10, checkpoint 2: extend this re-export with `Writer` and `WriterUsed`.

/// Day 1, checkpoint 3: add the bounds that connect an array to its scalar and builder forms.
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

/// Day 1, checkpoint 3: add the reciprocal bounds and item relationship for append-only builders.
pub trait ArrayBuilder {
    type Array;
    type RefItem<'a>;
    fn with_capacity(capacity: usize) -> Self;
    fn push(&mut self, value: Option<Self::RefItem<'_>>);
    fn finish(self) -> Self::Array;
}

/// The two erased array variants visible at the start of Day 1.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrayImpl {
    Int32(I32Array),
    String(StringArray),
    // Day 2: add the remaining primitive and Decimal variants.
    // Day 12: add `List(ListArray)`.
}

// Day 1, checkpoint 4: add erased dispatch for Int32 and String plus their explicit owned `From`,
// owned `TryFrom`, and borrowed `TryFrom` conversions. Keep these two families handwritten;
// Day 2 introduces catalog-driven generation when the physical-family inventory scales.
