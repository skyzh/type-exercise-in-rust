use crate::{Array, ArrayBuilder};

/// A compact teaching representation for nullable fixed-width values.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    values: Vec<Option<T>>,
}

/// The append-only builder for [`PrimitiveArray`].
#[derive(Debug)]
pub struct PrimitiveArrayBuilder<T> {
    values: Vec<Option<T>>,
}

pub type I32Array = PrimitiveArray<i32>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;

impl Array for I32Array {
    type Builder = I32ArrayBuilder;
    type OwnedItem = i32;
    type RefItem<'a> = i32;

    fn get(&self, row: usize) -> Option<Self::RefItem<'_>> {
        self.values[row]
    }

    fn len(&self) -> usize {
        self.values.len()
    }
}

impl ArrayBuilder for I32ArrayBuilder {
    type Array = I32Array;

    fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: Option<i32>) {
        self.values.push(value);
    }

    fn finish(self) -> Self::Array {
        PrimitiveArray {
            values: self.values,
        }
    }
}
