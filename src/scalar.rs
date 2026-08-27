use std::fmt::Debug;

use crate::variant_catalog::for_each_physical_family;
use crate::{Array, PhysicalType, TypeMismatch};

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

pub trait ScalarRef<'a>:
    Debug + Copy + 'a + TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch> + Into<ScalarRefImpl<'a>>
{
    type ArrayType: Array<RefItem<'a> = Self>;
    type ScalarType: Scalar<RefType<'a> = Self, ArrayType = Self::ArrayType>;
    fn to_owned_scalar(self) -> Self::ScalarType;
}

macro_rules! define_scalar_erasure {
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),+ $(,)?) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum ScalarImpl {
            $($variant($owned)),+
        }

        impl ScalarImpl {
            pub fn physical_type(&self) -> PhysicalType {
                match self {
                    $(Self::$variant(_) => PhysicalType::$variant),+
                }
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum ScalarRefImpl<'a> {
            $($variant($borrowed)),+
        }

        impl ScalarRefImpl<'_> {
            pub fn physical_type(&self) -> PhysicalType {
                match self {
                    $(Self::$variant(_) => PhysicalType::$variant),+
                }
            }

            pub fn to_owned_scalar(self) -> ScalarImpl {
                match self {
                    $(Self::$variant(value) => ScalarImpl::$variant(value.to_owned_scalar())),+
                }
            }
        }

        $(define_scalar_family!($kind, $variant, $array, $owned, $borrowed);)+
    };
}

macro_rules! define_scalar_family {
    (copy, $variant:ident, $array:ident, $owned:ty, $borrowed:ty) => {
        impl Scalar for $owned {
            const PHYSICAL_TYPE: PhysicalType = PhysicalType::$variant;
            type ArrayType = crate::$array;
            type RefType<'a> = $borrowed;
            fn as_scalar_ref(&self) -> Self::RefType<'_> {
                *self
            }
        }

        impl ScalarRef<'_> for $borrowed {
            type ArrayType = crate::$array;
            type ScalarType = $owned;
            fn to_owned_scalar(self) -> Self::ScalarType {
                self
            }
        }

        impl From<$owned> for ScalarImpl {
            fn from(value: $owned) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<ScalarImpl> for $owned {
            type Error = TypeMismatch;
            fn try_from(value: ScalarImpl) -> Result<Self, Self::Error> {
                match value {
                    ScalarImpl::$variant(value) => Ok(value),
                    other => Err(TypeMismatch {
                        expected: PhysicalType::$variant,
                        actual: other.physical_type(),
                    }),
                }
            }
        }

        impl<'a> From<$borrowed> for ScalarRefImpl<'a> {
            fn from(value: $borrowed) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<ScalarRefImpl<'_>> for $borrowed {
            type Error = TypeMismatch;
            fn try_from(value: ScalarRefImpl<'_>) -> Result<Self, Self::Error> {
                match value {
                    ScalarRefImpl::$variant(value) => Ok(value),
                    other => Err(TypeMismatch {
                        expected: PhysicalType::$variant,
                        actual: other.physical_type(),
                    }),
                }
            }
        }
    };
    (borrowed, $variant:ident, $array:ident, $owned:ty, $borrowed:ty) => {
        impl Scalar for $owned {
            const PHYSICAL_TYPE: PhysicalType = PhysicalType::$variant;
            type ArrayType = crate::$array;
            type RefType<'a> = &'a str;
            fn as_scalar_ref(&self) -> Self::RefType<'_> {
                self.as_str()
            }
        }

        impl<'a> ScalarRef<'a> for &'a str {
            type ArrayType = crate::$array;
            type ScalarType = $owned;
            fn to_owned_scalar(self) -> Self::ScalarType {
                self.to_owned()
            }
        }

        impl From<$owned> for ScalarImpl {
            fn from(value: $owned) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<ScalarImpl> for $owned {
            type Error = TypeMismatch;
            fn try_from(value: ScalarImpl) -> Result<Self, Self::Error> {
                match value {
                    ScalarImpl::$variant(value) => Ok(value),
                    other => Err(TypeMismatch {
                        expected: PhysicalType::$variant,
                        actual: other.physical_type(),
                    }),
                }
            }
        }

        impl<'a> From<&'a str> for ScalarRefImpl<'a> {
            fn from(value: &'a str) -> Self {
                Self::$variant(value)
            }
        }

        impl<'a> TryFrom<ScalarRefImpl<'a>> for &'a str {
            type Error = TypeMismatch;
            fn try_from(value: ScalarRefImpl<'a>) -> Result<Self, Self::Error> {
                match value {
                    ScalarRefImpl::$variant(value) => Ok(value),
                    other => Err(TypeMismatch {
                        expected: PhysicalType::$variant,
                        actual: other.physical_type(),
                    }),
                }
            }
        }
    };
}

for_each_physical_family!(define_scalar_erasure);
