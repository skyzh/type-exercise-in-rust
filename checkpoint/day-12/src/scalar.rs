use std::fmt::Debug;

use crate::variant_catalog::for_each_physical_family;
use crate::{
    Array, Decimal, DecimalError, ListError, ListScalar, ListScalarRef, PhysicalType, TypeMismatch,
};

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

macro_rules! erased_scalar_physical_type {
    (copy, $variant:ident, $value:ident) => {{
        let _ = $value;
        PhysicalType::$variant
    }};
    (borrowed, $variant:ident, $value:ident) => {{
        let _ = $value;
        PhysicalType::$variant
    }};
    (decimal, $variant:ident, $value:ident) => {
        PhysicalType::Decimal($value.decimal_type())
    };
}

macro_rules! erased_scalar_to_owned {
    (copy, $variant:ident, $value:ident) => {
        ScalarImpl::$variant($value.to_owned_scalar())
    };
    (borrowed, $variant:ident, $value:ident) => {
        ScalarImpl::$variant($value.to_owned_scalar())
    };
    (decimal, $variant:ident, $value:ident) => {
        ScalarImpl::$variant($value)
    };
}

macro_rules! define_scalar_erasure {
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),+ $(,)?) => {
        /// An owned scalar whose concrete type is known only at runtime.
        #[derive(Clone, Debug, PartialEq)]
        pub enum ScalarImpl {
            $($variant($owned)),+,
            List(ListScalar),
        }

        impl ScalarImpl {
            pub fn physical_type(&self) -> PhysicalType {
                match self {
                    $(Self::$variant(value) => erased_scalar_physical_type!($kind, $variant, value)),+,
                    Self::List(value) => PhysicalType::List(Box::new(value.element_type())),
                }
            }
        }

        /// A borrowed scalar whose concrete type is known only at runtime.
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum ScalarRefImpl<'a> {
            $($variant($borrowed)),+,
            List(ListScalarRef<'a>),
        }

        impl ScalarRefImpl<'_> {
            pub fn physical_type(&self) -> PhysicalType {
                match self {
                    $(Self::$variant(value) => erased_scalar_physical_type!($kind, $variant, value)),+,
                    Self::List(value) => PhysicalType::List(Box::new(value.element_type())),
                }
            }

            pub fn to_owned_scalar(self) -> ScalarImpl {
                match self {
                    $(Self::$variant(value) => erased_scalar_to_owned!($kind, $variant, value)),+,
                    Self::List(value) => ScalarImpl::List(
                        value
                            .to_owned_scalar()
                            .expect("a checked List scalar reference has a valid range"),
                    ),
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
    (decimal, $variant:ident, $array:ident, $owned:ty, $borrowed:ty) => {
        impl From<Decimal> for ScalarImpl {
            fn from(value: Decimal) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<ScalarImpl> for Decimal {
            type Error = DecimalError;

            fn try_from(value: ScalarImpl) -> Result<Self, Self::Error> {
                match value {
                    ScalarImpl::$variant(value) => Ok(value),
                    other => Err(DecimalError::ExpectedDecimal {
                        actual: other.physical_type(),
                    }),
                }
            }
        }

        impl<'a> From<Decimal> for ScalarRefImpl<'a> {
            fn from(value: Decimal) -> Self {
                Self::$variant(value)
            }
        }

        impl TryFrom<ScalarRefImpl<'_>> for Decimal {
            type Error = DecimalError;

            fn try_from(value: ScalarRefImpl<'_>) -> Result<Self, Self::Error> {
                match value {
                    ScalarRefImpl::$variant(value) => Ok(value),
                    other => Err(DecimalError::ExpectedDecimal {
                        actual: other.physical_type(),
                    }),
                }
            }
        }
    };
}

for_each_physical_family!(define_scalar_erasure);

impl From<ListScalar> for ScalarImpl {
    fn from(value: ListScalar) -> Self {
        Self::List(value)
    }
}

impl TryFrom<ScalarImpl> for ListScalar {
    type Error = ListError;

    fn try_from(value: ScalarImpl) -> Result<Self, Self::Error> {
        match value {
            ScalarImpl::List(value) => Ok(value),
            other => Err(ListError::ExpectedList {
                actual: other.physical_type(),
            }),
        }
    }
}

impl<'a> TryFrom<&'a ScalarImpl> for &'a ListScalar {
    type Error = ListError;

    fn try_from(value: &'a ScalarImpl) -> Result<Self, Self::Error> {
        match value {
            ScalarImpl::List(value) => Ok(value),
            other => Err(ListError::ExpectedList {
                actual: other.physical_type(),
            }),
        }
    }
}

impl<'a> From<ListScalarRef<'a>> for ScalarRefImpl<'a> {
    fn from(value: ListScalarRef<'a>) -> Self {
        Self::List(value)
    }
}

impl<'a> TryFrom<ScalarRefImpl<'a>> for ListScalarRef<'a> {
    type Error = ListError;

    fn try_from(value: ScalarRefImpl<'a>) -> Result<Self, Self::Error> {
        match value {
            ScalarRefImpl::List(value) => Ok(value),
            other => Err(ListError::ExpectedList {
                actual: other.physical_type(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScalarImpl;

    #[test]
    fn distinguishes_owned_scalar_variants() {
        assert_eq!(ScalarImpl::Int32(7), ScalarImpl::Int32(7));
        assert_eq!(
            ScalarImpl::String("rust".to_owned()),
            ScalarImpl::String("rust".to_owned())
        );
        assert_ne!(ScalarImpl::Int32(7), ScalarImpl::String("7".to_owned()));
    }
}
