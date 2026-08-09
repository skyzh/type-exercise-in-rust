use crate::{Array, ArrayBuilder};

/// A compact teaching representation for nullable fixed-width values.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    values: Vec<T>,
    validity: Vec<bool>,
    null_count: usize,
}

/// A checked view proving that every slot in a primitive array is valid.
#[derive(Clone, Copy, Debug)]
pub struct NonNullPrimitiveArray<'a, T> {
    array: &'a PrimitiveArray<T>,
}

/// The append-only builder for [`PrimitiveArray`].
#[derive(Debug)]
pub struct PrimitiveArrayBuilder<T> {
    values: Vec<T>,
    validity: Vec<bool>,
    null_count: usize,
}

pub type I32Array = PrimitiveArray<i32>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;
pub type F64Array = PrimitiveArray<f64>;
pub type F64ArrayBuilder = PrimitiveArrayBuilder<f64>;

impl<T> PrimitiveArray<T> {
    pub fn from_values(values: Vec<T>) -> Self {
        Self {
            validity: vec![true; values.len()],
            values,
            null_count: 0,
        }
    }

    pub fn null_count(&self) -> usize {
        self.null_count
    }

    pub fn as_non_null(&self) -> Option<NonNullPrimitiveArray<'_, T>> {
        (self.null_count == 0).then_some(NonNullPrimitiveArray { array: self })
    }
}

impl<'a, T> NonNullPrimitiveArray<'a, T> {
    pub fn values(self) -> &'a [T] {
        &self.array.values
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
            validity: Vec::with_capacity(capacity),
            null_count: 0,
        }
    }

    fn push(&mut self, value: Option<i32>) {
        match value {
            Some(value) => {
                self.values.push(value);
                self.validity.push(true);
            }
            None => {
                self.values.push(0);
                self.validity.push(false);
                self.null_count += 1;
            }
        }
    }

    fn finish(self) -> Self::Array {
        PrimitiveArray {
            values: self.values,
            validity: self.validity,
            null_count: self.null_count,
        }
    }
}

impl Array for F64Array {
    type Builder = F64ArrayBuilder;
    type OwnedItem = f64;
    type RefItem<'a> = f64;

    fn get(&self, row: usize) -> Option<Self::RefItem<'_>> {
        self.validity[row].then_some(self.values[row])
    }

    fn len(&self) -> usize {
        self.values.len()
    }
}

impl ArrayBuilder for F64ArrayBuilder {
    type Array = F64Array;

    fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            validity: Vec::with_capacity(capacity),
            null_count: 0,
        }
    }

    fn push(&mut self, value: Option<f64>) {
        match value {
            Some(value) => {
                self.values.push(value);
                self.validity.push(true);
            }
            None => {
                self.values.push(0.0);
                self.validity.push(false);
                self.null_count += 1;
            }
        }
    }

    fn finish(self) -> Self::Array {
        PrimitiveArray {
            values: self.values,
            validity: self.validity,
            null_count: self.null_count,
        }
    }
}
