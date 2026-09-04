mod primitive_array;
mod string_array;

// Checkpoint 1: uncomment after implementing Decimal metadata and storage.
// mod decimal_array;
// pub use decimal_array::{DecimalArray, DecimalArrayBuilder};

// A later checkpoint introduces List storage; keep `ListArrayBuilder` private to that module.
// mod list_array;
// pub use list_array::{ListArray, ListError, ListScalar, ListScalarRef};

pub use primitive_array::{I32Array, I32ArrayBuilder, PrimitiveArray, PrimitiveArrayBuilder};
// Checkpoint 1: extend the primitive re-export with I16, I64, Bool, F32, and F64 arrays/builders.
pub use string_array::{StringArray, StringArrayBuilder};

/// Checkpoint 1: add the bounds connecting an array to its scalar and builder forms.
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

/// Checkpoint 1: add the reciprocal bounds and item relationship for append-only builders.
pub trait ArrayBuilder {
    type Array;
    type RefItem<'a>;
    fn with_capacity(capacity: usize) -> Self;
    fn push(&mut self, value: Option<Self::RefItem<'_>>);
    fn finish(self) -> Self::Array;
}

/// Checkpoint 1: extend this starter pair to every non-List physical family.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrayImpl {
    Int32(I32Array),
    String(StringArray),
    // Add the remaining primitive and Decimal variants.
    // A later checkpoint adds List.
}

// Checkpoint 1: add erased dispatch plus owned From/TryFrom and borrowed TryFrom conversions.
// Generate the repeated family connections from the shared catalog.
