//! Primitive array and array builders.
//!
//! This module implements array for primitive types, like `i32` and `f32`.

use bitvec::prelude::BitVec;

use super::{Array, ArrayBuilder, ArrayImpl, ArrayIterator};
use crate::scalar::{Scalar, ScalarRef};
use crate::TypeMismatch;

/// A type that is primitive, such as `i32` and `i64`.
pub trait PrimitiveType: Scalar + Default {}

pub type I16Array = PrimitiveArray<i16>;
pub type I32Array = PrimitiveArray<i32>;
pub type I64Array = PrimitiveArray<i64>;
pub type F32Array = PrimitiveArray<f32>;
pub type F64Array = PrimitiveArray<f64>;
pub type BoolArray = PrimitiveArray<bool>;

pub type I16ArrayBuilder = PrimitiveArrayBuilder<i16>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;
pub type I64ArrayBuilder = PrimitiveArrayBuilder<i64>;
pub type F32ArrayBuilder = PrimitiveArrayBuilder<f32>;
pub type F64ArrayBuilder = PrimitiveArrayBuilder<f64>;
pub type BoolArrayBuilder = PrimitiveArrayBuilder<bool>;

impl PrimitiveType for i16 {}
impl PrimitiveType for i32 {}
impl PrimitiveType for i64 {}
impl PrimitiveType for f32 {}
impl PrimitiveType for f64 {}
impl PrimitiveType for bool {}

/// An [`Array`] that stores [`PrimitiveType`] items.
///
/// This array contains two parts: the value of each item, and the null bitmap of each item.
/// For example, if we create an [`Array`] of `[Some(1), None, Some(2)]`, it will be stored as
/// follows:
///
/// ```plain
/// data: [1, 0, 2]
/// bitmap: [true, false, true]
/// ```
///
/// We store the bitmap apart from data, so as to reduce memory footprint compared with
/// `Vec<Option<T>>`.
pub struct PrimitiveArray<T: PrimitiveType> {
    /// The actual data of this array.
    data: Vec<T>,

    /// The null bitmap of this array.
    bitmap: BitVec,
}

impl<T> Array for PrimitiveArray<T>
where
    T: PrimitiveType,
    T: Scalar<ArrayType = Self>,
    for<'a> T: ScalarRef<'a, ScalarType = T, ArrayType = Self>,
    for<'a> T: Scalar<RefType<'a> = T>,
    Self: Into<ArrayImpl>,
    Self: TryFrom<ArrayImpl, Error = TypeMismatch>,
{
    type Builder = PrimitiveArrayBuilder<T>;

    type OwnedItem = T;

    /// For `PrimitiveType`, we can always get the value from the array with little overhead.
    /// Therefore, we do not use the `'a` lifetime here, and simply copy the value to the user when
    /// calling `get`.
    type RefItem<'a> = T;

    fn get(&self, idx: usize) -> Option<T> {
        if self.bitmap[idx] {
            Some(self.data[idx])
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn iter(&self) -> ArrayIterator<Self> {
        ArrayIterator::new(self)
    }
}

/// [`ArrayBuilder`] for [`PrimitiveType`].
pub struct PrimitiveArrayBuilder<T: PrimitiveType> {
    /// The actual data of this array.
    data: Vec<T>,

    /// The null bitmap of this array.
    bitmap: BitVec,
}

impl<T> ArrayBuilder for PrimitiveArrayBuilder<T>
where
    T: PrimitiveType,
    T: Scalar<ArrayType = PrimitiveArray<T>>,
    for<'a> T: ScalarRef<'a, ScalarType = T, ArrayType = PrimitiveArray<T>>,
    for<'a> T: Scalar<RefType<'a> = T>,
    PrimitiveArray<T>: Into<ArrayImpl>,
    PrimitiveArray<T>: TryFrom<ArrayImpl, Error = TypeMismatch>,
{
    type Array = PrimitiveArray<T>;

    fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            bitmap: BitVec::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: Option<T>) {
        match value {
            Some(v) => {
                self.data.push(v);
                self.bitmap.push(true);
            }
            None => {
                self.data.push(T::default());
                self.bitmap.push(false);
            }
        }
    }

    fn finish(self) -> Self::Array {
        PrimitiveArray {
            data: self.data,
            bitmap: self.bitmap,
        }
    }
}
