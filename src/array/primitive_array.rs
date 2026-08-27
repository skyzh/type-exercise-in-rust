use bitvec::vec::BitVec;

use crate::{Array, ArrayBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    values: Vec<T>,
    validity: BitVec,
}

#[derive(Debug)]
pub struct PrimitiveArrayBuilder<T> {
    values: Vec<T>,
    validity: BitVec,
}

pub type I32Array = PrimitiveArray<i32>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;

impl<T> PrimitiveArray<T> {
    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn validity(&self) -> &BitVec {
        &self.validity
    }
}

impl Array for I32Array {
    type Builder = I32ArrayBuilder;
    type OwnedItem = i32;
    type RefItem<'a> = i32;

    fn get(&self, row: usize) -> Option<Self::RefItem<'_>> {
        self.validity[row].then_some(self.values[row])
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
            validity: BitVec::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: Option<i32>) {
        self.values.push(value.unwrap_or_default());
        self.validity.push(value.is_some());
    }

    fn finish(self) -> Self::Array {
        PrimitiveArray {
            values: self.values,
            validity: self.validity,
        }
    }
}

// Day 2 adds six explicit aliases, a private supported-primitive marker, and one generic
// Array/ArrayBuilder implementation instead of generating identical implementations with macros.
// Day 10 keeps this one representation and moves the checked non-null proof to column metadata.
