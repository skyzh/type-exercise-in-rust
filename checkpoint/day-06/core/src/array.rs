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

use anyhow::anyhow;
use std::fmt::Debug;

use crate::variant_catalog::for_each_physical_family;
use crate::{PhysicalType, Scalar, ScalarRef, ScalarRefImpl, TypeMismatch};

macro_rules! erased_array_physical_type {
    (copy, $variant:ident, $array:ident) => {{
        let _ = $array;
        PhysicalType::$variant
    }};
    (borrowed, $variant:ident, $array:ident) => {{
        let _ = $array;
        PhysicalType::$variant
    }};
    (decimal, $variant:ident, $array:ident) => {
        PhysicalType::Decimal($array.decimal_type())
    };
}

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

    fn from_slice(values: &[Option<Self::RefItem<'_>>]) -> Self {
        let mut builder = Self::Builder::with_capacity(values.len());
        for value in values {
            builder.push(*value);
        }
        builder.finish()
    }
}

pub trait ArrayBuilder: Sized {
    type Array: Array<Builder = Self>;
    fn with_capacity(capacity: usize) -> Self;
    fn push(&mut self, value: Option<<Self::Array as Array>::RefItem<'_>>);
    fn finish(self) -> Self::Array;
}

macro_rules! define_array_erasure {
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),+ $(,)?) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum ArrayImpl {
            $($variant($array)),+
        }

        impl ArrayImpl {
            pub fn physical_type(&self) -> PhysicalType {
                match self {
                    $(Self::$variant(array) => erased_array_physical_type!($kind, $variant, array)),+
                }
            }

            pub fn len(&self) -> usize {
                match self {
                    $(Self::$variant(array) => array.len()),+
                }
            }

            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'_>> {
                if row >= self.len() {
                    return None;
                }
                match self {
                    $(Self::$variant(array) => array.get(row).map(ScalarRefImpl::$variant)),+
                }
            }
        }

        $(define_array_family!($kind, $variant, $array);)+
    };
}

macro_rules! define_array_family {
    (copy, $variant:ident, $array:ident) => {
        impl From<$array> for ArrayImpl {
            fn from(array: $array) -> Self {
                Self::$variant(array)
            }
        }

        impl TryFrom<ArrayImpl> for $array {
            type Error = TypeMismatch;
            fn try_from(array: ArrayImpl) -> Result<Self, Self::Error> {
                match array {
                    ArrayImpl::$variant(array) => Ok(array),
                    other => Err(TypeMismatch {
                        expected: PhysicalType::$variant,
                        actual: other.physical_type(),
                    }),
                }
            }
        }

        impl<'a> TryFrom<&'a ArrayImpl> for &'a $array {
            type Error = TypeMismatch;
            fn try_from(array: &'a ArrayImpl) -> Result<Self, Self::Error> {
                match array {
                    ArrayImpl::$variant(array) => Ok(array),
                    other => Err(TypeMismatch {
                        expected: PhysicalType::$variant,
                        actual: other.physical_type(),
                    }),
                }
            }
        }
    };
    (borrowed, $variant:ident, $array:ident) => {
        define_array_family!(copy, $variant, $array);
    };
    (decimal, $variant:ident, $array:ident) => {
        impl From<$array> for ArrayImpl {
            fn from(array: $array) -> Self {
                Self::$variant(array)
            }
        }

        impl TryFrom<ArrayImpl> for $array {
            type Error = anyhow::Error;
            fn try_from(array: ArrayImpl) -> Result<Self, Self::Error> {
                match array {
                    ArrayImpl::$variant(array) => Ok(array),
                    other => Err(anyhow!(
                        "expected a Decimal value, got {:?}",
                        other.physical_type()
                    )),
                }
            }
        }

        impl<'a> TryFrom<&'a ArrayImpl> for &'a $array {
            type Error = anyhow::Error;
            fn try_from(array: &'a ArrayImpl) -> Result<Self, Self::Error> {
                match array {
                    ArrayImpl::$variant(array) => Ok(array),
                    other => Err(anyhow!(
                        "expected a Decimal value, got {:?}",
                        other.physical_type()
                    )),
                }
            }
        }
    };
}

for_each_physical_family!(define_array_erasure);

// Day 7 adds a checked non-null primitive view. Day 12 adds List erasure and slicing. Day 13
// replaces the iterator adapter with a private concrete iterator.
