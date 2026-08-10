use std::marker::PhantomData;

use bitvec::vec::BitVec;

use crate::{Array, ArrayBuilder};

/// Day 1, checkpoint 2: implement flat Int32 values plus packed validity.
/// Day 2, checkpoint 1: generalize the same layout to the remaining primitive families.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    marker: PhantomData<T>,
}

#[derive(Clone, Debug, PartialEq)]
/// Day 1, checkpoint 2: implement append-only primitive construction.
pub struct PrimitiveArrayBuilder<T> {
    marker: PhantomData<T>,
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
    pub fn values(&self) -> &[T] {
        todo!("store flat fixed-width values in Days 1–2")
    }
    pub fn validity(&self) -> &BitVec {
        todo!("store packed validity in Days 1–2")
    }
}

macro_rules! declare_primitive_array {
    ($type:ty) => {
        impl Array for PrimitiveArray<$type> {
            type Builder = PrimitiveArrayBuilder<$type>;
            type OwnedItem = $type;
            type RefItem<'a> = $type;
            fn get(&self, _: usize) -> Option<Self::RefItem<'_>> {
                todo!("read primitive rows in Days 1–2")
            }
            fn len(&self) -> usize {
                todo!("report primitive row count in Days 1–2")
            }
        }
        impl ArrayBuilder for PrimitiveArrayBuilder<$type> {
            type Array = PrimitiveArray<$type>;
            fn with_capacity(_: usize) -> Self {
                todo!("allocate primitive buffers in Days 1–2")
            }
            fn push(&mut self, _: Option<$type>) {
                todo!("append primitive rows in Days 1–2")
            }
            fn finish(self) -> Self::Array {
                todo!("finish primitive arrays in Days 1–2")
            }
        }
    };
}

declare_primitive_array!(i16);
declare_primitive_array!(i32);
declare_primitive_array!(i64);
declare_primitive_array!(bool);
declare_primitive_array!(f32);
declare_primitive_array!(f64);
