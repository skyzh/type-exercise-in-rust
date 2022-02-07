// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Primitive array and array builders.
//!
//! This module implements array for primitive types, like `i32` and `f32`.

use std::fmt::Debug;

use bitvec::prelude::BitVec;

use super::{Array, ArrayBuilder, ArrayIterator};

/// A type that is primitive, such as `i32` and `i64`.
pub trait PrimitiveType: Copy + Send + Sync + Default + Debug + 'static {}

pub type I32Array = PrimitiveArray<i32>;
pub type F32Array = PrimitiveArray<f32>;

impl PrimitiveType for i32 {}
impl PrimitiveType for f32 {}

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

impl<T: PrimitiveType> Array for PrimitiveArray<T> {
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

impl<T: PrimitiveType> ArrayBuilder for PrimitiveArrayBuilder<T> {
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
