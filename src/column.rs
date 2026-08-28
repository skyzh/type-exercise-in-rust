use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{Array, ArrayImpl, PhysicalType, Scalar, ScalarRefImpl, TypeMismatch};

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
    Indexed {
        indices: &'a [Option<usize>],
        values: &'a ArrayImpl,
    },
}

/// An indexed row selected a value outside the values array.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidIndex {
    pub row: usize,
    pub index: usize,
    pub values_len: usize,
}

impl Display for InvalidIndex {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "index {} at row {} is out of bounds for a values array of length {}",
            self.index, self.row, self.values_len
        )
    }
}

impl Error for InvalidIndex {}

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

    pub fn indexed(
        indices: &'a [Option<usize>],
        values: &'a ArrayImpl,
    ) -> Result<Self, InvalidIndex> {
        if let Some((row, index)) = indices
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(row, index)| index.map(|index| (row, index)))
            .find(|(_, index)| *index >= values.len())
        {
            return Err(InvalidIndex {
                row,
                index,
                values_len: values.len(),
            });
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
            ColumnViewImplKind::Indexed { indices, values } => {
                indices[row].and_then(|index| values.get(index))
            }
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
    Indexed {
        indices: &'a [Option<usize>],
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
            ColumnViewKind::Array(array) => {
                let array: &'a S::ArrayType = array;
                array.get(row)
            }
            ColumnViewKind::Constant { value, .. } => *value,
            ColumnViewKind::Indexed { indices, values } => {
                let values: &'a S::ArrayType = values;
                indices[row].and_then(|index| values.get(index))
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

#[cfg(test)]
mod tests {
    use super::ColumnViewImpl;
    use crate::{
        ArrayImpl, Decimal, DecimalArray, DecimalType, PhysicalType, ScalarImpl, ScalarRefImpl,
    };

    #[test]
    fn decimal_array_constant_null_and_indexed_keep_exact_metadata() {
        let decimal_type = DecimalType::try_new(8, 2).unwrap();
        let value = Decimal::try_new(12_345, decimal_type).unwrap();
        let array: ArrayImpl = DecimalArray::try_from_slice(decimal_type, &[Some(value), None])
            .unwrap()
            .into();
        let exact = PhysicalType::Decimal(decimal_type);

        let array_view = ColumnViewImpl::array(&array);
        assert_eq!(array_view.physical_type(), exact);
        assert_eq!(array_view.get(0), Some(ScalarRefImpl::Decimal(value)));
        assert_eq!(array_view.get(1), None);

        let constant = ColumnViewImpl::constant(ScalarRefImpl::Decimal(value), 2);
        assert_eq!(constant.physical_type(), exact);
        assert_eq!(constant.get(1), Some(ScalarRefImpl::Decimal(value)));

        let null = ColumnViewImpl::null(exact.clone(), 2);
        assert_eq!(null.physical_type(), exact);
        assert_eq!(null.get(0), None);

        let indices = [Some(1), Some(0), None];
        let indexed = ColumnViewImpl::indexed(&indices, &array).unwrap();
        assert_eq!(indexed.physical_type(), exact);
        assert_eq!(indexed.get(0), None);
        assert_eq!(indexed.get(1), Some(ScalarRefImpl::Decimal(value)));
        assert_eq!(indexed.get(2), None);

        assert_eq!(ScalarImpl::from(value).physical_type(), exact);
    }
}
