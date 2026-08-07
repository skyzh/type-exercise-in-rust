use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{Array, ArrayImpl, PhysicalType, Scalar, ScalarRefImpl, TypeMismatch};

/// A borrowed column whose scalar and array types are known only at runtime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnViewImpl<'a> {
    kind: ColumnViewImplKind<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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
            ColumnViewImplKind::Constant { physical_type, .. } => *physical_type,
            ColumnViewImplKind::Dictionary { values, .. } => values.physical_type(),
        }
    }
}

impl<'a> From<&'a ArrayImpl> for ColumnViewImpl<'a> {
    fn from(array: &'a ArrayImpl) -> Self {
        Self::array(array)
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
