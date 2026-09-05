use anyhow::{Result, anyhow};

use crate::{
    Array, ArrayImpl, ListArray, ListError, ListScalarRef, PhysicalType, Scalar, ScalarRefImpl,
    TypeMismatch,
};

/// A borrowed column whose physical family is known at runtime.
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
    Indexed {
        indices: &'a [u32],
        values: &'a ArrayImpl,
    },
}

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

    pub fn indexed(indices: &'a [u32], values: &'a ArrayImpl) -> Result<Self> {
        if let Some((row, index)) = indices
            .iter()
            .copied()
            .enumerate()
            .find(|(_, index)| *index as usize >= values.len())
        {
            return Err(anyhow!(
                "index {index} at row {row} is out of bounds for a values array of length {}",
                values.len()
            ));
        }

        Ok(Self {
            kind: ColumnViewImplKind::Indexed { indices, values },
        })
    }

    pub fn len(&self) -> usize {
        match &self.kind {
            ColumnViewImplKind::Array(array) => array.len(),
            ColumnViewImplKind::Constant { len, .. } => *len,
            ColumnViewImplKind::Indexed { indices, .. } => indices.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn physical_type(&self) -> PhysicalType {
        match &self.kind {
            ColumnViewImplKind::Array(array) => array.physical_type(),
            ColumnViewImplKind::Constant { physical_type, .. } => physical_type.clone(),
            ColumnViewImplKind::Indexed { values, .. } => values.physical_type(),
        }
    }

    pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'a>> {
        assert!(row < self.len(), "column view row out of bounds");
        match &self.kind {
            ColumnViewImplKind::Array(array) => array.get(row),
            ColumnViewImplKind::Constant { value, .. } => *value,
            ColumnViewImplKind::Indexed { indices, values } => values.get(indices[row] as usize),
        }
    }

    /// Check the List child type once and retain the same borrowed column shape.
    pub fn try_as_list(self, element_type: PhysicalType) -> Result<ListColumnView<'a>, ListError> {
        if matches!(element_type, PhysicalType::List(_)) {
            return Err(ListError::NestedList);
        }
        let expected = PhysicalType::List(Box::new(element_type));
        let actual = self.physical_type();
        if actual != expected {
            return Err(TypeMismatch { expected, actual }.into());
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
                        }
                        .into());
                    }
                };
                Ok(ListColumnView {
                    kind: ListColumnViewKind::Constant { value, len },
                })
            }
            ColumnViewImplKind::Indexed {
                indices,
                values: ArrayImpl::List(values),
            } => Ok(ListColumnView {
                kind: ListColumnViewKind::Indexed { indices, values },
            }),
            ColumnViewImplKind::Array(array) => Err(TypeMismatch {
                expected,
                actual: array.physical_type(),
            }
            .into()),
            ColumnViewImplKind::Indexed { values, .. } => Err(TypeMismatch {
                expected,
                actual: values.physical_type(),
            }
            .into()),
        }
    }
}

/// A borrowed List column whose child physical type was checked once.
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
    Indexed {
        indices: &'a [u32],
        values: &'a ListArray,
    },
}

impl<'a> ListColumnView<'a> {
    pub fn len(&self) -> usize {
        match &self.kind {
            ListColumnViewKind::Array(array) => array.len(),
            ListColumnViewKind::Constant { len, .. } => *len,
            ListColumnViewKind::Indexed { indices, .. } => indices.len(),
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
            ListColumnViewKind::Indexed { indices, values } => values.get(indices[row] as usize),
        }
    }
}

/// A borrowed column whose scalar family has been checked once.
#[derive(Debug)]
pub struct ColumnView<'a, S: Scalar> {
    pub(crate) kind: ColumnViewKind<'a, S>,
}

#[derive(Debug)]
pub(crate) enum ColumnViewKind<'a, S: Scalar> {
    Array(&'a S::ArrayType),
    Constant {
        value: Option<S::RefType<'a>>,
        len: usize,
    },
    Indexed {
        indices: &'a [u32],
        values: &'a S::ArrayType,
    },
}

impl<'a, S: Scalar> ColumnView<'a, S> {
    pub fn len(&self) -> usize {
        match &self.kind {
            ColumnViewKind::Array(array) => array.len(),
            ColumnViewKind::Constant { len, .. } => *len,
            ColumnViewKind::Indexed { indices, .. } => indices.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        assert!(row < self.len(), "column view row out of bounds");
        match &self.kind {
            ColumnViewKind::Array(array) => array.get(row),
            ColumnViewKind::Constant { value, .. } => *value,
            ColumnViewKind::Indexed { indices, values } => values.get(indices[row] as usize),
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
            ColumnViewImplKind::Indexed { indices, values } => Ok(Self {
                kind: ColumnViewKind::Indexed {
                    indices,
                    values: values.try_into()?,
                },
            }),
        }
    }
}
