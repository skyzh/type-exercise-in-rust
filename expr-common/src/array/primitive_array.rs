// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Primitive array and array builders.
//!
//! This module implements array for primitive types, like `i32` and `f32`.

use bitvec::prelude::BitVec;
use rust_decimal::Decimal;

use super::{Array, ArrayBuilder, ArrayImpl, ArrayIterator};
use crate::TypeMismatch;
use crate::scalar::{Scalar, ScalarRef};

/// A type that is primitive, such as `i32` and `i64`.
pub trait PrimitiveType: Scalar + Default {}

pub type I16Array = PrimitiveArray<i16>;
pub type I32Array = PrimitiveArray<i32>;
pub type I64Array = PrimitiveArray<i64>;
pub type F32Array = PrimitiveArray<f32>;
pub type F64Array = PrimitiveArray<f64>;
pub type BoolArray = PrimitiveArray<bool>;
pub type DecimalArray = PrimitiveArray<Decimal>;

pub type I16ArrayBuilder = PrimitiveArrayBuilder<i16>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;
pub type I64ArrayBuilder = PrimitiveArrayBuilder<i64>;
pub type F32ArrayBuilder = PrimitiveArrayBuilder<f32>;
pub type F64ArrayBuilder = PrimitiveArrayBuilder<f64>;
pub type BoolArrayBuilder = PrimitiveArrayBuilder<bool>;
pub type DecimalArrayBuilder = PrimitiveArrayBuilder<Decimal>;

impl PrimitiveType for i16 {}
impl PrimitiveType for i32 {}
impl PrimitiveType for i64 {}
impl PrimitiveType for f32 {}
impl PrimitiveType for f64 {}
impl PrimitiveType for bool {}
impl PrimitiveType for Decimal {}

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
#[derive(Clone)]
pub struct PrimitiveArray<T: PrimitiveType> {
    /// The actual data of this array.
    data: Vec<T>,

    /// The null bitmap of this array.
    bitmap: BitVec,

    /// Cached so the executor can select an all-valid loop once per batch.
    null_count: usize,
}

/// A primitive array proven to contain no nulls.
///
/// Construct this wrapper once per batch with [`PrimitiveArray::as_non_null`], then use
/// [`values`](Self::values) without checking validity inside the hot loop.
#[derive(Clone, Copy)]
pub struct NonNullPrimitiveArray<'a, T: PrimitiveType>(&'a PrimitiveArray<T>);

impl<'a, T: PrimitiveType> NonNullPrimitiveArray<'a, T> {
    /// Contiguous values with the same lifetime as the borrowed array.
    pub fn values(self) -> &'a [T] {
        &self.0.data
    }
}

impl<T: PrimitiveType> PrimitiveArray<T> {
    /// Construct an array whose rows are all valid, initializing validity in bulk.
    pub fn from_values(data: Vec<T>) -> Self {
        let bitmap = BitVec::repeat(true, data.len());
        Self {
            data,
            bitmap,
            null_count: 0,
        }
    }

    /// Prove once that the array has no nulls.
    pub fn as_non_null(&self) -> Option<NonNullPrimitiveArray<'_, T>> {
        (self.null_count == 0).then_some(NonNullPrimitiveArray(self))
    }

    /// Whether at least one row is null.
    pub fn has_nulls(&self) -> bool {
        self.null_count() != 0
    }

    /// Number of null rows, cached for constant-time batch dispatch.
    pub fn null_count(&self) -> usize {
        self.null_count
    }
}

impl<T> Array for PrimitiveArray<T>
where
    T: PrimitiveType,
    T: Scalar<ArrayType = Self>,
    for<'a> T: ScalarRef<'a, ScalarType = T, ArrayType = Self>,
    for<'a> T: Scalar<RefType<'a> = T>,
    Self: Into<ArrayImpl>,
    Self: TryFrom<ArrayImpl, Error = TypeMismatch>,
    Self: std::fmt::Debug,
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

    fn iter(&self) -> ArrayIterator<'_, Self> {
        ArrayIterator::new(self)
    }
}

/// [`ArrayBuilder`] for [`PrimitiveType`].
pub struct PrimitiveArrayBuilder<T: PrimitiveType> {
    /// The actual data of this array.
    data: Vec<T>,

    /// The null bitmap of this array.
    bitmap: BitVec,

    /// Number of nulls pushed into this builder.
    null_count: usize,
}

impl<T> ArrayBuilder for PrimitiveArrayBuilder<T>
where
    T: PrimitiveType,
    T: Scalar<ArrayType = PrimitiveArray<T>>,
    for<'a> T: ScalarRef<'a, ScalarType = T, ArrayType = PrimitiveArray<T>>,
    for<'a> T: Scalar<RefType<'a> = T>,
    PrimitiveArray<T>: Into<ArrayImpl>,
    PrimitiveArray<T>: TryFrom<ArrayImpl, Error = TypeMismatch>,
    PrimitiveArray<T>: std::fmt::Debug,
{
    type Array = PrimitiveArray<T>;

    fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            bitmap: BitVec::with_capacity(capacity),
            null_count: 0,
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
                self.null_count += 1;
            }
        }
    }

    fn finish(self) -> Self::Array {
        PrimitiveArray {
            data: self.data,
            bitmap: self.bitmap,
            null_count: self.null_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_all_valid_arrays() {
        let array = I32Array::from_slice(&[Some(1), Some(2), Some(3)]);
        assert!(!array.has_nulls());
        assert_eq!(array.as_non_null().unwrap().values(), &[1, 2, 3]);
    }

    #[test]
    fn rejects_non_null_proof_when_a_null_exists() {
        let array = I32Array::from_slice(&[Some(1), None, Some(3)]);
        assert!(array.has_nulls());
        assert!(array.as_non_null().is_none());
        assert_eq!(array.get(0), Some(1));
        assert_eq!(array.get(1), None);
        assert_eq!(array.get(2), Some(3));
    }
}
