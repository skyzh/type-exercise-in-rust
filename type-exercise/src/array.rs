mod iterator;
mod list_array;
mod primitive_array;
mod string_array;

use iterator::ArrayIterator;
pub use list_array::{ListArray, ListError, ListScalar, ListScalarRef};
pub use primitive_array::{
    BoolArray, BoolArrayBuilder, DecimalArray, DecimalArrayBuilder, F32Array, F32ArrayBuilder,
    F64Array, F64ArrayBuilder, I16Array, I16ArrayBuilder, I32Array, I32ArrayBuilder, I64Array,
    I64ArrayBuilder, NonNullPrimitiveArray, PrimitiveArray, PrimitiveArrayBuilder,
};
pub use string_array::{StringArray, StringArrayBuilder};

use std::fmt::Debug;

use crate::variant_catalog::for_each_physical_family;
use crate::{Decimal, PhysicalType, Scalar, ScalarImpl, ScalarRef, ScalarRefImpl, TypeMismatch};

macro_rules! build_array_family_from_scalars {
    ($values:expr, $variant:ident, $array:ident, $owned:ty) => {{
        let typed = $values
            .into_iter()
            .map(|value| value.map(<$owned>::try_from).transpose())
            .collect::<Result<Vec<_>, _>>()?;
        let borrowed = typed
            .iter()
            .map(|value| value.as_ref().map(Scalar::as_scalar_ref))
            .collect::<Vec<_>>();
        Ok(Self::$variant($array::from_slice(&borrowed)))
    }};
}

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

    /// Iterate without exposing the concrete iterator type.
    ///
    /// ```compile_fail
    /// use type_exercise::ArrayIterator;
    /// ```
    fn iter<'a>(&'a self) -> impl Iterator<Item = Option<Self::RefItem<'a>>> + 'a {
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

macro_rules! define_array_erasure {
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),+ $(,)?) => {
        /// A physical array whose concrete type is known only at runtime.
        #[derive(Clone, Debug, PartialEq)]
        pub enum ArrayImpl {
            $($variant($array)),+,
            List(ListArray),
        }

        impl ArrayImpl {
            pub fn physical_type(&self) -> PhysicalType {
                match self {
                    $(Self::$variant(_) => PhysicalType::$variant),+,
                    Self::List(array) => PhysicalType::List(Box::new(array.element_type())),
                }
            }

            pub fn len(&self) -> usize {
                match self {
                    $(Self::$variant(array) => array.len()),+,
                    Self::List(array) => array.len(),
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
                    $(Self::$variant(array) => array.get(row).map(ScalarRefImpl::$variant)),+,
                    Self::List(array) => array
                        .get(row)
                        .ok()
                        .flatten()
                        .map(ScalarRefImpl::List),
                }
            }

            pub(crate) fn slice(&self, start: usize, end: usize) -> Result<Self, ListError> {
                if start > end || end > self.len() {
                    return Err(ListError::RangeOutOfBounds {
                        start,
                        end,
                        len: self.len(),
                    });
                }
                match self {
                    $(Self::$variant(array) => {
                        let values = (start..end).map(|row| array.get(row)).collect::<Vec<_>>();
                        Ok(Self::$variant($array::from_slice(&values)))
                    }),+,
                    Self::List(_) => Err(ListError::NestedList),
                }
            }

            pub(crate) fn try_from_scalars(
                physical_type: &PhysicalType,
                values: Vec<Option<ScalarImpl>>,
            ) -> Result<Self, ListError> {
                match physical_type {
                    PhysicalType::Int16 => build_array_family_from_scalars!(values, Int16, I16Array, i16),
                    PhysicalType::Int32 => build_array_family_from_scalars!(values, Int32, I32Array, i32),
                    PhysicalType::Int64 => build_array_family_from_scalars!(values, Int64, I64Array, i64),
                    PhysicalType::Bool => build_array_family_from_scalars!(values, Bool, BoolArray, bool),
                    PhysicalType::Float32 => build_array_family_from_scalars!(values, Float32, F32Array, f32),
                    PhysicalType::Float64 => build_array_family_from_scalars!(values, Float64, F64Array, f64),
                    PhysicalType::String => build_array_family_from_scalars!(values, String, StringArray, String),
                    PhysicalType::Decimal => build_array_family_from_scalars!(values, Decimal, DecimalArray, Decimal),
                    PhysicalType::List(_) => Err(ListError::NestedList),
                }
            }
        }

        $(define_array_family!($variant, $array);)+
    };
}

macro_rules! define_array_family {
    ($variant:ident, $array:ident) => {
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
}

for_each_physical_family!(define_array_erasure);

impl From<ListArray> for ArrayImpl {
    fn from(array: ListArray) -> Self {
        Self::List(array)
    }
}

impl TryFrom<ArrayImpl> for ListArray {
    type Error = ListError;

    fn try_from(array: ArrayImpl) -> Result<Self, Self::Error> {
        match array {
            ArrayImpl::List(array) => Ok(array),
            other => Err(ListError::ExpectedList {
                actual: other.physical_type(),
            }),
        }
    }
}

impl<'a> TryFrom<&'a ArrayImpl> for &'a ListArray {
    type Error = ListError;

    fn try_from(array: &'a ArrayImpl) -> Result<Self, Self::Error> {
        match array {
            ArrayImpl::List(array) => Ok(array),
            other => Err(ListError::ExpectedList {
                actual: other.physical_type(),
            }),
        }
    }
}
