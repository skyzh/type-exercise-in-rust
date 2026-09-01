use crate::{Array, ArrayBuilder, ArrayImpl, Scalar, ScalarRef, TypeMismatch};
use bitvec::vec::BitVec;

/// A compact teaching representation for nullable fixed-width values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveArray<T> {
    values: Vec<T>,
    validity: BitVec,
}

/// The append-only builder for [`PrimitiveArray`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveArrayBuilder<T> {
    values: Vec<T>,
    validity: BitVec,
}

pub type I16Array = PrimitiveArray<i16>;
pub type I16ArrayBuilder = PrimitiveArrayBuilder<i16>;
pub type I32Array = PrimitiveArray<i32>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;
pub type I64Array = PrimitiveArray<i64>;
pub type I64ArrayBuilder = PrimitiveArrayBuilder<i64>;
pub type BoolArray = PrimitiveArray<bool>;
pub type BoolArrayBuilder = PrimitiveArrayBuilder<bool>;
pub type F32Array = PrimitiveArray<f32>;
pub type F32ArrayBuilder = PrimitiveArrayBuilder<f32>;
pub type F64Array = PrimitiveArray<f64>;
pub type F64ArrayBuilder = PrimitiveArrayBuilder<f64>;

impl<T> PrimitiveArray<T> {
    pub fn from_values(values: Vec<T>) -> Self {
        Self {
            validity: BitVec::repeat(true, values.len()),
            values,
        }
    }

    pub(crate) fn from_raw_parts(values: Vec<T>, validity: BitVec) -> Self {
        debug_assert_eq!(values.len(), validity.len());
        Self { values, validity }
    }

    /// The contiguous fixed-width value buffer.
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// The packed row-validity bitmap.
    pub fn validity(&self) -> &BitVec {
        &self.validity
    }

    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
}

impl<T> PrimitiveArrayBuilder<T> {
    pub(crate) fn with_raw_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            validity: BitVec::with_capacity(capacity),
        }
    }

    pub(crate) fn push_raw(&mut self, value: T, valid: bool) {
        self.values.push(value);
        self.validity.push(valid);
    }

    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn finish_raw(self) -> PrimitiveArray<T> {
        PrimitiveArray::from_raw_parts(self.values, self.validity)
    }
}

impl<T> Array for PrimitiveArray<T>
where
    T: Scalar<ArrayType = Self> + Copy + Default,
    for<'a> T: ScalarRef<'a, ScalarType = T, ArrayType = Self>,
    for<'a> T: Scalar<RefType<'a> = T>,
    Self: Into<ArrayImpl> + TryFrom<ArrayImpl, Error = TypeMismatch>,
{
    type Builder = PrimitiveArrayBuilder<T>;
    type OwnedItem = T;
    type RefItem<'a> = T;

    fn get(&self, row: usize) -> Option<Self::RefItem<'_>> {
        self.validity[row].then_some(self.values[row])
    }

    fn len(&self) -> usize {
        self.values.len()
    }
}

impl<T> ArrayBuilder for PrimitiveArrayBuilder<T>
where
    T: Scalar<ArrayType = PrimitiveArray<T>> + Copy + Default,
    for<'a> T: ScalarRef<'a, ScalarType = T, ArrayType = PrimitiveArray<T>>,
    for<'a> T: Scalar<RefType<'a> = T>,
    PrimitiveArray<T>: Into<ArrayImpl> + TryFrom<ArrayImpl, Error = TypeMismatch>,
{
    type Array = PrimitiveArray<T>;

    fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            validity: BitVec::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: Option<T>) {
        match value {
            Some(value) => {
                self.values.push(value);
                self.validity.push(true);
            }
            None => {
                self.values.push(T::default());
                self.validity.push(false);
            }
        }
    }

    fn finish(self) -> Self::Array {
        PrimitiveArray {
            values: self.values,
            validity: self.validity,
        }
    }
}
