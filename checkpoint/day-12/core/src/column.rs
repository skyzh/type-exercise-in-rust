use anyhow::{Result, anyhow};

use crate::{
    Array, ArrayImpl, ListArray, ListError, ListScalarRef, Nullability, PhysicalType, Scalar,
    ScalarRefImpl, TypeMismatch,
};

/// A borrowed column whose scalar and array types are known only at runtime.
///
/// The public wrapper keeps construction checked while the private kind prevents
/// callers from bypassing those checks, and it stores batch nullability once.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnViewImpl<'a> {
    kind: ColumnViewImplKind<'a>,
    nullability: Nullability,
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum DenseI32Column<'a> {
    Array(&'a crate::I32Array),
    Constant { value: i32, len: usize },
}

impl DenseI32Column<'_> {
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Array(array) => array.values().len(),
            Self::Constant { len, .. } => len,
        }
    }
}

impl<'a> ColumnViewImpl<'a> {
    pub fn array(array: &'a ArrayImpl) -> Self {
        Self {
            kind: ColumnViewImplKind::Array(array),
            nullability: Nullability::Nullable,
        }
    }

    pub fn try_non_null_array(array: &'a ArrayImpl) -> Option<Self> {
        (0..array.len())
            .all(|row| array.get(row).is_some())
            .then_some(Self {
                kind: ColumnViewImplKind::Array(array),
                nullability: Nullability::NonNull,
            })
    }

    pub fn constant(value: ScalarRefImpl<'a>, len: usize) -> Self {
        Self {
            kind: ColumnViewImplKind::Constant {
                value: Some(value),
                physical_type: value.physical_type(),
                len,
            },
            nullability: Nullability::NonNull,
        }
    }

    pub fn null(physical_type: PhysicalType, len: usize) -> Self {
        Self {
            kind: ColumnViewImplKind::Constant {
                value: None,
                physical_type,
                len,
            },
            nullability: Nullability::Nullable,
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
            nullability: Nullability::Nullable,
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

    pub fn nullability(&self) -> Nullability {
        self.nullability
    }

    /// Return one erased scalar after the caller has checked the row bound.
    pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'a>> {
        assert!(row < self.len(), "column view row out of bounds");
        match &self.kind {
            ColumnViewImplKind::Array(array) => array.get(row),
            ColumnViewImplKind::Constant { value, .. } => *value,
            ColumnViewImplKind::Indexed { indices, values } => values.get(indices[row] as usize),
        }
    }

    pub(crate) fn as_dense_i32(&self) -> Option<DenseI32Column<'a>> {
        if self.nullability != Nullability::NonNull {
            return None;
        }
        match &self.kind {
            ColumnViewImplKind::Array(ArrayImpl::Int32(array)) => {
                Some(DenseI32Column::Array(array))
            }
            ColumnViewImplKind::Constant {
                value: Some(ScalarRefImpl::Int32(value)),
                len,
                ..
            } => Some(DenseI32Column::Constant {
                value: *value,
                len: *len,
            }),
            _ => None,
        }
    }

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
    Indexed {
        indices: &'a [u32],
        values: &'a S::ArrayType,
    },
}

impl<'a, S: Scalar> ColumnView<'a, S> {
    // The typed surface stays limited to operations used by the course.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match &self.kind {
            ColumnViewKind::Array(array) => array.len(),
            ColumnViewKind::Constant { len, .. } => *len,
            ColumnViewKind::Indexed { indices, .. } => indices.len(),
        }
    }

    pub fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        assert!(row < self.len(), "column view row out of bounds");
        match &self.kind {
            ColumnViewKind::Array(array) => {
                let array: &'a S::ArrayType = array;
                array.get(row)
            }
            ColumnViewKind::Constant { value, .. } => *value,
            ColumnViewKind::Indexed { indices, values } => {
                let values: &'a S::ArrayType = values;
                values.get(indices[row] as usize)
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
            ColumnViewImplKind::Indexed { indices, values } => Ok(Self {
                kind: ColumnViewKind::Indexed {
                    indices,
                    values: values.try_into()?,
                },
            }),
        }
    }
}
