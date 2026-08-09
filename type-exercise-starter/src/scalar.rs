use std::fmt::Debug;

use crate::{Array, Decimal, DecimalError, PhysicalType, TypeMismatch};

/// Day 1 target: connect an owned scalar to its borrowed and array forms.
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

/// Day 1 target: describe the copyable value borrowed from a scalar or array.
pub trait ScalarRef<'a>:
    Debug + Copy + 'a + TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch> + Into<ScalarRefImpl<'a>>
{
    type ArrayType: Array<RefItem<'a> = Self>;
    type ScalarType: Scalar<RefType<'a> = Self, ArrayType = Self::ArrayType>;
    fn to_owned_scalar(self) -> Self::ScalarType;
}

/// Days 1–2 target: the runtime-owned scalar variants.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarImpl {
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Bool(bool),
    Float32(f32),
    Float64(f64),
    String(String),
    Decimal(Decimal),
}

/// Days 1–2 target: the runtime-borrowed scalar variants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarRefImpl<'a> {
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Bool(bool),
    Float32(f32),
    Float64(f64),
    String(&'a str),
    Decimal(Decimal),
}

impl ScalarImpl {
    pub fn try_decimal(&self, _: crate::DecimalType) -> Result<Decimal, DecimalError> {
        todo!("check exact Decimal metadata in Day 2")
    }
}

impl ScalarRefImpl<'_> {
    pub fn try_decimal(self, _: crate::DecimalType) -> Result<Decimal, DecimalError> {
        todo!("check exact borrowed Decimal metadata in Day 2")
    }
}

macro_rules! declare_copy_scalar_family {
    ($owned:ty, $variant:ident, $array:ident) => {
        impl Scalar for $owned {
            const PHYSICAL_TYPE: PhysicalType = PhysicalType::$variant;
            type ArrayType = crate::$array;
            type RefType<'a> = $owned;
            fn as_scalar_ref(&self) -> Self::RefType<'_> {
                todo!("implement the scalar family in Day 2")
            }
        }
        impl ScalarRef<'_> for $owned {
            type ArrayType = crate::$array;
            type ScalarType = $owned;
            fn to_owned_scalar(self) -> Self::ScalarType {
                todo!("implement the scalar family in Day 2")
            }
        }
        impl From<$owned> for ScalarImpl {
            fn from(_: $owned) -> Self {
                todo!("implement scalar erasure in Day 2")
            }
        }
        impl TryFrom<ScalarImpl> for $owned {
            type Error = TypeMismatch;
            fn try_from(_: ScalarImpl) -> Result<Self, Self::Error> {
                todo!("implement checked scalar recovery in Day 2")
            }
        }
        impl<'a> From<$owned> for ScalarRefImpl<'a> {
            fn from(_: $owned) -> Self {
                todo!("implement borrowed scalar erasure in Day 2")
            }
        }
        impl TryFrom<ScalarRefImpl<'_>> for $owned {
            type Error = TypeMismatch;
            fn try_from(_: ScalarRefImpl<'_>) -> Result<Self, Self::Error> {
                todo!("implement checked borrowed-scalar recovery in Day 2")
            }
        }
    };
}

declare_copy_scalar_family!(i16, Int16, I16Array);
declare_copy_scalar_family!(i32, Int32, I32Array);
declare_copy_scalar_family!(i64, Int64, I64Array);
declare_copy_scalar_family!(bool, Bool, BoolArray);
declare_copy_scalar_family!(f32, Float32, F32Array);
declare_copy_scalar_family!(f64, Float64, F64Array);

impl Scalar for String {
    const PHYSICAL_TYPE: PhysicalType = PhysicalType::String;
    type ArrayType = crate::StringArray;
    type RefType<'a> = &'a str;
    fn as_scalar_ref(&self) -> Self::RefType<'_> {
        todo!("implement the borrowed String family in Day 1")
    }
}

impl<'a> ScalarRef<'a> for &'a str {
    type ArrayType = crate::StringArray;
    type ScalarType = String;
    fn to_owned_scalar(self) -> Self::ScalarType {
        todo!("implement the borrowed String family in Day 1")
    }
}

impl From<String> for ScalarImpl {
    fn from(_: String) -> Self {
        todo!("implement String erasure in Day 1")
    }
}

impl TryFrom<ScalarImpl> for String {
    type Error = TypeMismatch;
    fn try_from(_: ScalarImpl) -> Result<Self, Self::Error> {
        todo!("implement checked String recovery in Day 1")
    }
}

impl<'a> From<&'a str> for ScalarRefImpl<'a> {
    fn from(_: &'a str) -> Self {
        todo!("implement borrowed String erasure in Day 1")
    }
}

impl<'a> TryFrom<ScalarRefImpl<'a>> for &'a str {
    type Error = TypeMismatch;
    fn try_from(_: ScalarRefImpl<'a>) -> Result<Self, Self::Error> {
        todo!("implement checked borrowed String recovery in Day 1")
    }
}

impl From<Decimal> for ScalarImpl {
    fn from(_: Decimal) -> Self {
        todo!("implement Decimal erasure in Day 2")
    }
}

impl TryFrom<ScalarImpl> for Decimal {
    type Error = DecimalError;
    fn try_from(_: ScalarImpl) -> Result<Self, Self::Error> {
        todo!("implement checked Decimal recovery in Day 2")
    }
}

impl<'a> From<Decimal> for ScalarRefImpl<'a> {
    fn from(_: Decimal) -> Self {
        todo!("implement borrowed Decimal erasure in Day 2")
    }
}

impl TryFrom<ScalarRefImpl<'_>> for Decimal {
    type Error = DecimalError;
    fn try_from(_: ScalarRefImpl<'_>) -> Result<Self, Self::Error> {
        todo!("implement checked borrowed Decimal recovery in Day 2")
    }
}

#[cfg(test)]
mod tests {
    use super::ScalarImpl;

    #[test]
    fn starter_distinguishes_the_two_owned_scalar_variants() {
        assert_eq!(ScalarImpl::Int32(7), ScalarImpl::Int32(7));
        assert_eq!(
            ScalarImpl::String("rust".to_owned()),
            ScalarImpl::String("rust".to_owned())
        );
        assert_ne!(ScalarImpl::Int32(7), ScalarImpl::String("7".to_owned()));
    }
}
