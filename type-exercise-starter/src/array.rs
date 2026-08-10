mod decimal_array;
mod primitive_array;
mod string_array;

pub use decimal_array::{DecimalArray, DecimalArrayBuilder};
pub use primitive_array::{
    BoolArray, BoolArrayBuilder, F32Array, F32ArrayBuilder, F64Array, F64ArrayBuilder, I16Array,
    I16ArrayBuilder, I32Array, I32ArrayBuilder, I64Array, I64ArrayBuilder, PrimitiveArray,
    PrimitiveArrayBuilder,
};
pub use string_array::{StringArray, StringArrayBuilder};

use std::fmt::Debug;

use crate::{PhysicalType, Scalar, ScalarRef, ScalarRefImpl, TypeMismatch};

/// Day 1 target: one nullable physical value family.
pub trait Array:
    Debug + Clone + Sized + TryFrom<ArrayImpl, Error = TypeMismatch> + Into<ArrayImpl>
where
    for<'a> Self::OwnedItem: Scalar<ArrayType = Self, RefType<'a> = Self::RefItem<'a>>,
{
    type Builder: ArrayBuilder<Array = Self>;
    type OwnedItem: Scalar<ArrayType = Self>;
    type RefItem<'a>: ScalarRef<'a, ScalarType = Self::OwnedItem, ArrayType = Self>;
    fn get(&self, row: usize) -> Option<Self::RefItem<'_>>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn iter<'a>(&'a self) -> impl Iterator<Item = Option<Self::RefItem<'a>>> + 'a {
        (0..self.len()).map(|row| self.get(row))
    }
    fn from_slice(_: &[Option<Self::RefItem<'_>>]) -> Self {
        todo!("build arrays from nullable rows in Day 1")
    }
}

/// Day 1 target: append-only construction for one static family.
pub trait ArrayBuilder: Sized {
    type Array: Array<Builder = Self>;
    fn with_capacity(capacity: usize) -> Self;
    fn push(&mut self, value: Option<<Self::Array as Array>::RefItem<'_>>);
    fn finish(self) -> Self::Array;
}

/// Days 1–2 target: runtime-erased array variants.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrayImpl {
    Int16(I16Array),
    Int32(I32Array),
    Int64(I64Array),
    Bool(BoolArray),
    Float32(F32Array),
    Float64(F64Array),
    String(StringArray),
    Decimal(DecimalArray),
}

impl ArrayImpl {
    pub fn physical_type(&self) -> PhysicalType {
        todo!("implement array erasure in Days 1–2")
    }
    pub fn len(&self) -> usize {
        todo!("implement erased array length in Days 1–2")
    }
    pub fn is_empty(&self) -> bool {
        todo!("implement erased empty-array checks in Days 1–2")
    }
    pub fn get(&self, _: usize) -> Option<ScalarRefImpl<'_>> {
        todo!("implement erased array reads in Days 1–2")
    }
    pub fn try_decimal(&self, _: crate::DecimalType) -> Result<&DecimalArray, crate::DecimalError> {
        todo!("check exact Decimal array metadata in Day 2")
    }
}

macro_rules! declare_array_erasure {
    ($array:ident, $variant:ident) => {
        impl From<$array> for ArrayImpl {
            fn from(_: $array) -> Self {
                todo!("implement array erasure in Days 1–2")
            }
        }
        impl TryFrom<ArrayImpl> for $array {
            type Error = TypeMismatch;
            fn try_from(_: ArrayImpl) -> Result<Self, Self::Error> {
                todo!("implement checked array recovery in Days 1–2")
            }
        }
        impl<'a> TryFrom<&'a ArrayImpl> for &'a $array {
            type Error = TypeMismatch;
            fn try_from(_: &'a ArrayImpl) -> Result<Self, Self::Error> {
                todo!("implement checked borrowed-array recovery in Days 1–2")
            }
        }
    };
}

declare_array_erasure!(I16Array, Int16);
declare_array_erasure!(I32Array, Int32);
declare_array_erasure!(I64Array, Int64);
declare_array_erasure!(BoolArray, Bool);
declare_array_erasure!(F32Array, Float32);
declare_array_erasure!(F64Array, Float64);
declare_array_erasure!(StringArray, String);

impl From<DecimalArray> for ArrayImpl {
    fn from(_: DecimalArray) -> Self {
        todo!("implement Decimal array erasure in Day 2")
    }
}

impl TryFrom<ArrayImpl> for DecimalArray {
    type Error = crate::DecimalError;
    fn try_from(_: ArrayImpl) -> Result<Self, Self::Error> {
        todo!("implement checked Decimal array recovery in Day 2")
    }
}
