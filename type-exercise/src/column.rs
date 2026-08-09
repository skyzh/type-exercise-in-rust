use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    Array, ArrayImpl, ListArray, ListError, ListScalarRef, NonNullPrimitiveArray, PhysicalType,
    Scalar, ScalarRefImpl, TypeMismatch,
};

/// A borrowed column whose scalar and array types are known only at runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnViewImpl<'a> {
    kind: ColumnViewImplKind<'a>,
}

#[derive(Clone, Debug, PartialEq)]
enum ColumnViewImplKind<'a> {
    Array(&'a ArrayImpl),
    Constant {
        value: Option<ScalarRefImpl<'a>>,
        physical_type: PhysicalType,
        len: usize,
    },
    Dictionary {
        indices: &'a [Option<usize>],
        values: &'a ArrayImpl,
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum NonNullI32Column<'a> {
    Array(NonNullPrimitiveArray<'a, i32>),
    Constant { value: i32, len: usize },
}

impl NonNullI32Column<'_> {
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Array(array) => array.values().len(),
            Self::Constant { len, .. } => len,
        }
    }
}

/// A dictionary key selected a value outside the dictionary-values array.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDictionaryKey {
    pub row: usize,
    pub key: usize,
    pub dictionary_len: usize,
}

impl Display for InvalidDictionaryKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "dictionary key {} at row {} is out of bounds for a dictionary of length {}",
            self.key, self.row, self.dictionary_len
        )
    }
}

impl Error for InvalidDictionaryKey {}

impl<'a> ColumnViewImpl<'a> {
    pub fn array(array: &'a ArrayImpl) -> Self {
        Self {
            kind: ColumnViewImplKind::Array(array),
        }
    }

    pub fn constant(value: ScalarRefImpl<'a>, len: usize) -> Self {
        Self {
            kind: ColumnViewImplKind::Constant {
                value: Some(value),
                physical_type: value.physical_type(),
                len,
            },
        }
    }

    pub fn null(physical_type: PhysicalType, len: usize) -> Self {
        Self {
            kind: ColumnViewImplKind::Constant {
                value: None,
                physical_type,
                len,
            },
        }
    }

    pub fn dictionary(
        indices: &'a [Option<usize>],
        values: &'a ArrayImpl,
    ) -> Result<Self, InvalidDictionaryKey> {
        if let Some((row, key)) = indices
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(row, key)| key.map(|key| (row, key)))
            .find(|(_, key)| *key >= values.len())
        {
            return Err(InvalidDictionaryKey {
                row,
                key,
                dictionary_len: values.len(),
            });
        }

        Ok(Self {
            kind: ColumnViewImplKind::Dictionary { indices, values },
        })
    }

    pub fn len(&self) -> usize {
        match &self.kind {
            ColumnViewImplKind::Array(array) => array.len(),
            ColumnViewImplKind::Constant { len, .. } => *len,
            ColumnViewImplKind::Dictionary { indices, .. } => indices.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn physical_type(&self) -> PhysicalType {
        match &self.kind {
            ColumnViewImplKind::Array(array) => array.physical_type(),
            ColumnViewImplKind::Constant { physical_type, .. } => physical_type.clone(),
            ColumnViewImplKind::Dictionary { values, .. } => values.physical_type(),
        }
    }

    /// Return one erased scalar after the caller has checked the row bound.
    pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'a>> {
        assert!(row < self.len(), "column view row out of bounds");
        match &self.kind {
            ColumnViewImplKind::Array(array) => array.get(row),
            ColumnViewImplKind::Constant { value, .. } => *value,
            ColumnViewImplKind::Dictionary { indices, values } => {
                indices[row].and_then(|key| values.get(key))
            }
        }
    }

    pub(crate) fn as_non_null_i32(&self) -> Option<NonNullI32Column<'a>> {
        match &self.kind {
            ColumnViewImplKind::Array(ArrayImpl::Int32(array)) => {
                array.as_non_null().map(NonNullI32Column::Array)
            }
            ColumnViewImplKind::Constant {
                value: Some(ScalarRefImpl::Int32(value)),
                len,
                ..
            } => Some(NonNullI32Column::Constant {
                value: *value,
                len: *len,
            }),
            _ => None,
        }
    }

    pub fn try_as_list(
        self,
        element_type: PhysicalType,
    ) -> Result<ListColumnView<'a>, TypeMismatch> {
        let expected = PhysicalType::List(Box::new(element_type));
        let actual = self.physical_type();
        if actual != expected {
            return Err(TypeMismatch { expected, actual });
        }
        match self.kind {
            ColumnViewImplKind::Array(ArrayImpl::List(array)) => Ok(ListColumnView {
                kind: ListColumnViewKind::Array(array),
            }),
            ColumnViewImplKind::Constant { value, len, .. } => {
                let value = match value {
                    Some(ScalarRefImpl::List(value)) => Some(value),
                    None => None,
                    Some(other) => {
                        return Err(TypeMismatch {
                            expected,
                            actual: other.physical_type(),
                        });
                    }
                };
                Ok(ListColumnView {
                    kind: ListColumnViewKind::Constant { value, len },
                })
            }
            ColumnViewImplKind::Dictionary {
                indices,
                values: ArrayImpl::List(values),
            } => Ok(ListColumnView {
                kind: ListColumnViewKind::Dictionary { indices, values },
            }),
            ColumnViewImplKind::Array(array) => Err(TypeMismatch {
                expected,
                actual: array.physical_type(),
            }),
            ColumnViewImplKind::Dictionary { values, .. } => Err(TypeMismatch {
                expected,
                actual: values.physical_type(),
            }),
        }
    }
}

impl<'a> From<&'a ArrayImpl> for ColumnViewImpl<'a> {
    fn from(array: &'a ArrayImpl) -> Self {
        Self::array(array)
    }
}

/// A borrowed List column whose element physical type was checked once.
#[derive(Debug)]
pub struct ListColumnView<'a> {
    kind: ListColumnViewKind<'a>,
}

#[derive(Debug)]
enum ListColumnViewKind<'a> {
    Array(&'a ListArray),
    Constant {
        value: Option<ListScalarRef<'a>>,
        len: usize,
    },
    Dictionary {
        indices: &'a [Option<usize>],
        values: &'a ListArray,
    },
}

impl<'a> ListColumnView<'a> {
    pub fn len(&self) -> usize {
        match &self.kind {
            ListColumnViewKind::Array(array) => array.len(),
            ListColumnViewKind::Constant { len, .. } => *len,
            ListColumnViewKind::Dictionary { indices, .. } => indices.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, row: usize) -> Result<Option<ListScalarRef<'a>>, ListError> {
        if row >= self.len() {
            return Err(ListError::RowOutOfBounds {
                row,
                len: self.len(),
            });
        }
        match &self.kind {
            ListColumnViewKind::Array(array) => array.get(row),
            ListColumnViewKind::Constant { value, .. } => Ok(*value),
            ListColumnViewKind::Dictionary { indices, values } => {
                indices[row].map_or(Ok(None), |key| values.get(key))
            }
        }
    }
}

/// A borrowed logical column whose scalar family is known at compile time.
#[derive(Debug)]
pub struct ColumnView<'a, S: Scalar> {
    kind: ColumnViewKind<'a, S>,
}

#[derive(Debug)]
enum ColumnViewKind<'a, S: Scalar> {
    Array(&'a S::ArrayType),
    Constant {
        value: Option<S::RefType<'a>>,
        len: usize,
    },
    Dictionary {
        indices: &'a [Option<usize>],
        values: &'a S::ArrayType,
    },
}

impl<'a, S: Scalar> ColumnView<'a, S> {
    pub fn len(&self) -> usize {
        match &self.kind {
            ColumnViewKind::Array(array) => array.len(),
            ColumnViewKind::Constant { len, .. } => *len,
            ColumnViewKind::Dictionary { indices, .. } => indices.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        assert!(row < self.len(), "column view row out of bounds");
        match &self.kind {
            ColumnViewKind::Array(array) => {
                let array: &'a S::ArrayType = array;
                array.get(row)
            }
            ColumnViewKind::Constant { value, .. } => *value,
            ColumnViewKind::Dictionary { indices, values } => {
                let values: &'a S::ArrayType = values;
                indices[row].and_then(|key| values.get(key))
            }
        }
    }
}

impl<'a, S> TryFrom<ColumnViewImpl<'a>> for ColumnView<'a, S>
where
    S: Scalar,
    &'a S::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    S::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    type Error = TypeMismatch;

    fn try_from(view: ColumnViewImpl<'a>) -> Result<Self, Self::Error> {
        if view.physical_type() != S::PHYSICAL_TYPE {
            return Err(TypeMismatch {
                expected: S::PHYSICAL_TYPE,
                actual: view.physical_type(),
            });
        }

        match view.kind {
            ColumnViewImplKind::Array(array) => Ok(Self {
                kind: ColumnViewKind::Array(array.try_into()?),
            }),
            ColumnViewImplKind::Constant { value, len, .. } => Ok(Self {
                kind: ColumnViewKind::Constant {
                    value: value.map(TryInto::try_into).transpose()?,
                    len,
                },
            }),
            ColumnViewImplKind::Dictionary { indices, values } => Ok(Self {
                kind: ColumnViewKind::Dictionary {
                    indices,
                    values: values.try_into()?,
                },
            }),
        }
    }
}
