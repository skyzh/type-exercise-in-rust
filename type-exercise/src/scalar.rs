use std::fmt::Debug;

use crate::{Array, I32Array, PhysicalType, StringArray, TypeMismatch};

/// An owned scalar and its associated borrowed value and array representation.
pub trait Scalar:
    Debug + Clone + TryFrom<ScalarImpl, Error = TypeMismatch> + Into<ScalarImpl>
where
    for<'a> Self::ArrayType: Array<OwnedItem = Self, RefItem<'a> = Self::RefType<'a>>,
{
    const PHYSICAL_TYPE: PhysicalType;

    type ArrayType: Array<OwnedItem = Self>;
    type RefType<'a>: ScalarRef<'a, ScalarType = Self, ArrayType = Self::ArrayType>;

    fn as_scalar_ref(&self) -> Self::RefType<'_>;
}

/// A copyable value borrowed from an owned scalar or array.
pub trait ScalarRef<'a>:
    Debug + Copy + 'a + TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch> + Into<ScalarRefImpl<'a>>
{
    type ArrayType: Array<RefItem<'a> = Self>;
    type ScalarType: Scalar<RefType<'a> = Self, ArrayType = Self::ArrayType>;

    fn to_owned_scalar(self) -> Self::ScalarType;
}

/// An owned scalar whose concrete type is known only at runtime.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarImpl {
    Int32(i32),
    String(String),
}

impl ScalarImpl {
    pub fn physical_type(&self) -> PhysicalType {
        match self {
            Self::Int32(_) => PhysicalType::Int32,
            Self::String(_) => PhysicalType::String,
        }
    }
}

/// A borrowed scalar whose concrete type is known only at runtime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarRefImpl<'a> {
    Int32(i32),
    String(&'a str),
}

impl ScalarRefImpl<'_> {
    pub fn physical_type(&self) -> PhysicalType {
        match self {
            Self::Int32(_) => PhysicalType::Int32,
            Self::String(_) => PhysicalType::String,
        }
    }
}

impl Scalar for i32 {
    const PHYSICAL_TYPE: PhysicalType = PhysicalType::Int32;

    type ArrayType = I32Array;
    type RefType<'a> = i32;

    fn as_scalar_ref(&self) -> Self::RefType<'_> {
        *self
    }
}

impl ScalarRef<'_> for i32 {
    type ArrayType = I32Array;
    type ScalarType = i32;

    fn to_owned_scalar(self) -> Self::ScalarType {
        self
    }
}

impl Scalar for String {
    const PHYSICAL_TYPE: PhysicalType = PhysicalType::String;

    type ArrayType = StringArray;
    type RefType<'a> = &'a str;

    fn as_scalar_ref(&self) -> Self::RefType<'_> {
        self.as_str()
    }
}

impl<'a> ScalarRef<'a> for &'a str {
    type ArrayType = StringArray;
    type ScalarType = String;

    fn to_owned_scalar(self) -> Self::ScalarType {
        self.to_owned()
    }
}

impl From<i32> for ScalarImpl {
    fn from(value: i32) -> Self {
        Self::Int32(value)
    }
}

impl TryFrom<ScalarImpl> for i32 {
    type Error = TypeMismatch;

    fn try_from(value: ScalarImpl) -> Result<Self, Self::Error> {
        match value {
            ScalarImpl::Int32(value) => Ok(value),
            other => Err(TypeMismatch {
                expected: PhysicalType::Int32,
                actual: other.physical_type(),
            }),
        }
    }
}

impl From<String> for ScalarImpl {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl TryFrom<ScalarImpl> for String {
    type Error = TypeMismatch;

    fn try_from(value: ScalarImpl) -> Result<Self, Self::Error> {
        match value {
            ScalarImpl::String(value) => Ok(value),
            other => Err(TypeMismatch {
                expected: PhysicalType::String,
                actual: other.physical_type(),
            }),
        }
    }
}

impl<'a> From<i32> for ScalarRefImpl<'a> {
    fn from(value: i32) -> Self {
        Self::Int32(value)
    }
}

impl TryFrom<ScalarRefImpl<'_>> for i32 {
    type Error = TypeMismatch;

    fn try_from(value: ScalarRefImpl<'_>) -> Result<Self, Self::Error> {
        match value {
            ScalarRefImpl::Int32(value) => Ok(value),
            other => Err(TypeMismatch {
                expected: PhysicalType::Int32,
                actual: other.physical_type(),
            }),
        }
    }
}

impl<'a> From<&'a str> for ScalarRefImpl<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(value)
    }
}

impl<'a> TryFrom<ScalarRefImpl<'a>> for &'a str {
    type Error = TypeMismatch;

    fn try_from(value: ScalarRefImpl<'a>) -> Result<Self, Self::Error> {
        match value {
            ScalarRefImpl::String(value) => Ok(value),
            other => Err(TypeMismatch {
                expected: PhysicalType::String,
                actual: other.physical_type(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScalarImpl;

    #[test]
    fn distinguishes_the_two_owned_scalar_variants() {
        assert_eq!(ScalarImpl::Int32(7), ScalarImpl::Int32(7));
        assert_eq!(
            ScalarImpl::String("rust".to_owned()),
            ScalarImpl::String("rust".to_owned())
        );
        assert_ne!(ScalarImpl::Int32(7), ScalarImpl::String("7".to_owned()));
    }
}
