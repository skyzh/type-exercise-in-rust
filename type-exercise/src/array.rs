mod iterator;
mod primitive_array;
mod string_array;

pub use iterator::ArrayIterator;
pub use primitive_array::{
    I32Array, I32ArrayBuilder, NonNullPrimitiveArray, PrimitiveArray, PrimitiveArrayBuilder,
};
pub use string_array::{StringArray, StringArrayBuilder};

use std::fmt::Debug;

use crate::{PhysicalType, Scalar, ScalarRef, ScalarRefImpl, TypeMismatch};

/// A nullable collection of one physical value type.
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

    fn iter(&self) -> ArrayIterator<'_, Self> {
        ArrayIterator::new(self)
    }

    fn from_slice(values: &[Option<Self::RefItem<'_>>]) -> Self {
        let mut builder = Self::Builder::with_capacity(values.len());
        for value in values {
            builder.push(*value);
        }
        builder.finish()
    }
}

/// The append-only companion that constructs one concrete array type.
pub trait ArrayBuilder: Sized {
    type Array: Array<Builder = Self>;

    fn with_capacity(capacity: usize) -> Self;
    fn push(&mut self, value: Option<<Self::Array as Array>::RefItem<'_>>);
    fn finish(self) -> Self::Array;
}

/// A physical array whose concrete type is known only at runtime.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrayImpl {
    Int32(I32Array),
    String(StringArray),
}

impl ArrayImpl {
    pub fn physical_type(&self) -> PhysicalType {
        match self {
            Self::Int32(_) => PhysicalType::Int32,
            Self::String(_) => PhysicalType::String,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Int32(array) => array.len(),
            Self::String(array) => array.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'_>> {
        match self {
            Self::Int32(array) => array.get(row).map(ScalarRefImpl::Int32),
            Self::String(array) => array.get(row).map(ScalarRefImpl::String),
        }
    }
}

impl From<I32Array> for ArrayImpl {
    fn from(array: I32Array) -> Self {
        Self::Int32(array)
    }
}

impl TryFrom<ArrayImpl> for I32Array {
    type Error = TypeMismatch;

    fn try_from(array: ArrayImpl) -> Result<Self, Self::Error> {
        match array {
            ArrayImpl::Int32(array) => Ok(array),
            other => Err(TypeMismatch {
                expected: PhysicalType::Int32,
                actual: other.physical_type(),
            }),
        }
    }
}

impl<'a> TryFrom<&'a ArrayImpl> for &'a I32Array {
    type Error = TypeMismatch;

    fn try_from(array: &'a ArrayImpl) -> Result<Self, Self::Error> {
        match array {
            ArrayImpl::Int32(array) => Ok(array),
            other => Err(TypeMismatch {
                expected: PhysicalType::Int32,
                actual: other.physical_type(),
            }),
        }
    }
}

impl From<StringArray> for ArrayImpl {
    fn from(array: StringArray) -> Self {
        Self::String(array)
    }
}

impl TryFrom<ArrayImpl> for StringArray {
    type Error = TypeMismatch;

    fn try_from(array: ArrayImpl) -> Result<Self, Self::Error> {
        match array {
            ArrayImpl::String(array) => Ok(array),
            other => Err(TypeMismatch {
                expected: PhysicalType::String,
                actual: other.physical_type(),
            }),
        }
    }
}

impl<'a> TryFrom<&'a ArrayImpl> for &'a StringArray {
    type Error = TypeMismatch;

    fn try_from(array: &'a ArrayImpl) -> Result<Self, Self::Error> {
        match array {
            ArrayImpl::String(array) => Ok(array),
            other => Err(TypeMismatch {
                expected: PhysicalType::String,
                actual: other.physical_type(),
            }),
        }
    }
}
