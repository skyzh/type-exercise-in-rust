use anyhow::{Result, anyhow};
use bitvec::vec::BitVec;

use crate::{Array, ArrayImpl, PhysicalType, Scalar, ScalarRefImpl, TypeMismatch};

/// A borrowed column whose scalar and array types are known only at runtime.
///
/// The public wrapper keeps its representation enum private, so callers must use the checked
/// constructors instead of creating an unvalidated indexed view. Later chapters can also attach
/// batch-wide metadata here without repeating it in every representation variant.
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum RawI32Column<'a> {
    Array {
        values: &'a [i32],
        validity: &'a BitVec,
    },
    Constant {
        value: i32,
        valid: bool,
        len: usize,
    },
}

impl RawI32Column<'_> {
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Array { values, .. } => values.len(),
            Self::Constant { len, .. } => len,
        }
    }
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

    /// Return one erased scalar after the caller has checked the row bound.
    pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'a>> {
        assert!(row < self.len(), "column view row out of bounds");
        match &self.kind {
            ColumnViewImplKind::Array(array) => array.get(row),
            ColumnViewImplKind::Constant { value, .. } => *value,
            ColumnViewImplKind::Indexed { indices, values } => values.get(indices[row] as usize),
        }
    }

    pub(crate) fn as_raw_i32(&self) -> Option<RawI32Column<'_>> {
        match &self.kind {
            ColumnViewImplKind::Array(ArrayImpl::Int32(array)) => Some(RawI32Column::Array {
                values: array.values(),
                validity: array.validity(),
            }),
            ColumnViewImplKind::Constant { value, len, .. } => Some(RawI32Column::Constant {
                value: match value {
                    Some(ScalarRefImpl::Int32(value)) => *value,
                    None => 0,
                    Some(_) => return None,
                },
                valid: value.is_some(),
                len: *len,
            }),
            _ => None,
        }
    }

    pub(crate) fn is_indexed(&self) -> bool {
        matches!(self.kind, ColumnViewImplKind::Indexed { .. })
    }
}

/// A borrowed logical column whose scalar family is known at compile time.
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
