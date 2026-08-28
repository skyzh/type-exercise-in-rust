use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{ArrayImpl, PhysicalType, ScalarImpl, ScalarRefImpl, TypeMismatch};

/// A checked failure while constructing or slicing a one-level List value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ListError {
    NestedList,
    ExpectedList {
        actual: PhysicalType,
    },
    TypeMismatch(TypeMismatch),
    Decimal(String),
    OffsetCount {
        expected: usize,
        actual: usize,
    },
    OffsetMustStartAtZero {
        actual: usize,
    },
    OffsetOutOfOrder {
        row: usize,
        start: usize,
        end: usize,
    },
    FinalOffset {
        expected: usize,
        actual: usize,
    },
    NullRowHasValues {
        row: usize,
        start: usize,
        end: usize,
    },
    RowOutOfBounds {
        row: usize,
        len: usize,
    },
    RangeOutOfBounds {
        start: usize,
        end: usize,
        len: usize,
    },
}

impl Display for ListError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NestedList => formatter.write_str("nested List values are not supported"),
            Self::ExpectedList { actual } => {
                write!(formatter, "expected a List value, got {actual:?}")
            }
            Self::TypeMismatch(error) => Display::fmt(error, formatter),
            Self::Decimal(error) => Display::fmt(error, formatter),
            Self::OffsetCount { expected, actual } => write!(
                formatter,
                "list offset count mismatch: expected {expected}, got {actual}"
            ),
            Self::OffsetMustStartAtZero { actual } => {
                write!(formatter, "list offsets must start at 0, got {actual}")
            }
            Self::OffsetOutOfOrder { row, start, end } => write!(
                formatter,
                "list offsets are not monotone at row {row}: {start} > {end}"
            ),
            Self::FinalOffset { expected, actual } => write!(
                formatter,
                "list final offset mismatch: expected {expected}, got {actual}"
            ),
            Self::NullRowHasValues { row, start, end } => write!(
                formatter,
                "null list row {row} must repeat its offset, got {start}..{end}"
            ),
            Self::RowOutOfBounds { row, len } => {
                write!(
                    formatter,
                    "list row {row} is out of bounds for length {len}"
                )
            }
            Self::RangeOutOfBounds { start, end, len } => write!(
                formatter,
                "list range {start}..{end} is out of bounds for length {len}"
            ),
        }
    }
}

impl Error for ListError {}

impl From<TypeMismatch> for ListError {
    fn from(error: TypeMismatch) -> Self {
        Self::TypeMismatch(error)
    }
}

impl From<anyhow::Error> for ListError {
    fn from(error: anyhow::Error) -> Self {
        Self::Decimal(error.to_string())
    }
}

/// One owned, non-null List scalar.
#[derive(Clone, Debug, PartialEq)]
pub struct ListScalar {
    values: Box<ArrayImpl>,
}

impl ListScalar {
    pub fn try_new(values: ArrayImpl) -> Result<Self, ListError> {
        reject_nested(&values.physical_type())?;
        Ok(Self {
            values: Box::new(values),
        })
    }

    pub fn element_type(&self) -> PhysicalType {
        self.values.physical_type()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, index: usize) -> Result<Option<ScalarRefImpl<'_>>, ListError> {
        if index >= self.len() {
            return Err(ListError::RowOutOfBounds {
                row: index,
                len: self.len(),
            });
        }
        Ok(self.values.get(index))
    }

    pub fn as_list_ref(&self) -> ListScalarRef<'_> {
        ListScalarRef {
            values: &self.values,
            start: 0,
            end: self.values.len(),
        }
    }

    pub fn slice(&self, start: usize, end: usize) -> Result<ListScalar, ListError> {
        self.as_list_ref().slice(start, end)?.to_owned_scalar()
    }
}

/// One safely bounded, borrowed, non-null List scalar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListScalarRef<'a> {
    values: &'a ArrayImpl,
    start: usize,
    end: usize,
}

impl<'a> ListScalarRef<'a> {
    pub(crate) fn new(values: &'a ArrayImpl, start: usize, end: usize) -> Self {
        Self { values, start, end }
    }

    pub fn element_type(self) -> PhysicalType {
        self.values.physical_type()
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub fn get(self, index: usize) -> Result<Option<ScalarRefImpl<'a>>, ListError> {
        if index >= self.len() {
            return Err(ListError::RowOutOfBounds {
                row: index,
                len: self.len(),
            });
        }
        Ok(self.values.get(self.start + index))
    }

    pub fn slice(self, start: usize, end: usize) -> Result<Self, ListError> {
        if start > end || end > self.len() {
            return Err(ListError::RangeOutOfBounds {
                start,
                end,
                len: self.len(),
            });
        }
        Ok(Self::new(self.values, self.start + start, self.start + end))
    }

    pub fn to_owned_scalar(self) -> Result<ListScalar, ListError> {
        ListScalar::try_new(self.values.slice(self.start, self.end)?)
    }
}

/// A nullable, one-level List array with an explicit child physical type.
#[derive(Clone, Debug, PartialEq)]
pub struct ListArray {
    element_type: PhysicalType,
    values: Box<ArrayImpl>,
    offsets: Vec<usize>,
    validity: Vec<bool>,
}

impl ListArray {
    /// Validate externally supplied List storage before exposing any row.
    pub fn try_from_raw_parts(
        element_type: PhysicalType,
        values: ArrayImpl,
        offsets: Vec<usize>,
        validity: Vec<bool>,
    ) -> Result<Self, ListError> {
        reject_nested(&element_type)?;
        let actual_type = values.physical_type();
        if actual_type != element_type {
            return Err(TypeMismatch {
                expected: element_type,
                actual: actual_type,
            }
            .into());
        }
        let expected_offsets = validity.len() + 1;
        if offsets.len() != expected_offsets {
            return Err(ListError::OffsetCount {
                expected: expected_offsets,
                actual: offsets.len(),
            });
        }
        let first = offsets.first().copied().unwrap_or(usize::MAX);
        if first != 0 {
            return Err(ListError::OffsetMustStartAtZero { actual: first });
        }
        for (row, window) in offsets.windows(2).enumerate() {
            let start = window[0];
            let end = window[1];
            if start > end {
                return Err(ListError::OffsetOutOfOrder { row, start, end });
            }
            if !validity[row] && start != end {
                return Err(ListError::NullRowHasValues { row, start, end });
            }
        }
        let final_offset = offsets.last().copied().unwrap_or(usize::MAX);
        if final_offset != values.len() {
            return Err(ListError::FinalOffset {
                expected: values.len(),
                actual: final_offset,
            });
        }
        Ok(Self {
            element_type,
            values: Box::new(values),
            offsets,
            validity,
        })
    }

    /// Build a one-level List array without exposing its fallible append state.
    ///
    /// The element type is required up front, so zero-row and all-null arrays remain typed.
    pub fn try_from_rows<'a>(
        element_type: PhysicalType,
        rows: impl IntoIterator<Item = Option<ListScalarRef<'a>>>,
    ) -> Result<Self, ListError> {
        let rows = rows.into_iter().collect::<Vec<_>>();
        let mut builder = ListArrayBuilder::new(element_type, rows.len())?;
        for row in rows {
            builder.push(row)?;
        }
        builder.finish()
    }

    pub fn element_type(&self) -> PhysicalType {
        self.element_type.clone()
    }

    pub fn values(&self) -> &ArrayImpl {
        &self.values
    }

    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    pub fn validity(&self) -> &[bool] {
        &self.validity
    }

    pub fn len(&self) -> usize {
        self.validity.len()
    }

    pub fn is_empty(&self) -> bool {
        self.validity.is_empty()
    }

    pub fn get(&self, row: usize) -> Result<Option<ListScalarRef<'_>>, ListError> {
        if row >= self.len() {
            return Err(ListError::RowOutOfBounds {
                row,
                len: self.len(),
            });
        }
        if !self.validity[row] {
            return Ok(None);
        }
        Ok(Some(ListScalarRef::new(
            &self.values,
            self.offsets[row],
            self.offsets[row + 1],
        )))
    }

    pub fn slice(&self, start: usize, end: usize) -> Result<Self, ListError> {
        if start > end || end > self.len() {
            return Err(ListError::RangeOutOfBounds {
                start,
                end,
                len: self.len(),
            });
        }
        let child_start = self.offsets[start];
        let child_end = self.offsets[end];
        let offsets = self.offsets[start..=end]
            .iter()
            .map(|offset| offset - child_start)
            .collect();
        Self::try_from_raw_parts(
            self.element_type.clone(),
            self.values.slice(child_start, child_end)?,
            offsets,
            self.validity[start..end].to_vec(),
        )
    }
}

#[derive(Debug)]
pub(crate) struct ListArrayBuilder {
    element_type: PhysicalType,
    values: Vec<Option<ScalarImpl>>,
    offsets: Vec<usize>,
    validity: Vec<bool>,
}

impl ListArrayBuilder {
    pub(crate) fn new(element_type: PhysicalType, capacity: usize) -> Result<Self, ListError> {
        reject_nested(&element_type)?;
        let mut offsets = Vec::with_capacity(capacity + 1);
        offsets.push(0);
        Ok(Self {
            element_type,
            values: Vec::new(),
            offsets,
            validity: Vec::with_capacity(capacity),
        })
    }

    pub(crate) fn push(&mut self, value: Option<ListScalarRef<'_>>) -> Result<(), ListError> {
        let Some(value) = value else {
            self.validity.push(false);
            self.offsets.push(self.values.len());
            return Ok(());
        };
        let actual = value.element_type();
        if actual != self.element_type {
            return Err(TypeMismatch {
                expected: self.element_type.clone(),
                actual,
            }
            .into());
        }

        // Materialize the row before mutating the builder so an error cannot leak a prefix.
        let row_values = (0..value.len())
            .map(|index| {
                value
                    .get(index)
                    .map(|item| item.map(ScalarRefImpl::to_owned_scalar))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.values.extend(row_values);
        self.validity.push(true);
        self.offsets.push(self.values.len());
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<ListArray, ListError> {
        let values = ArrayImpl::try_from_scalars(&self.element_type, self.values)?;
        ListArray::try_from_raw_parts(self.element_type, values, self.offsets, self.validity)
    }
}

fn reject_nested(physical_type: &PhysicalType) -> Result<(), ListError> {
    if matches!(physical_type, PhysicalType::List(_)) {
        Err(ListError::NestedList)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ListArrayBuilder, ListError, ListScalar};
    use crate::{
        Array, Decimal, DecimalArray, DecimalType, I32Array, PhysicalType, StringArray,
        TypeMismatch,
    };

    #[test]
    fn failed_row_keeps_builder_state_unchanged() {
        let integers = ListScalar::try_new(I32Array::from_slice(&[Some(1)]).into()).unwrap();
        let strings =
            ListScalar::try_new(StringArray::from_slice(&[Some("wrong")]).into()).unwrap();
        let mut builder = ListArrayBuilder::new(PhysicalType::Int32, 2).unwrap();
        builder.push(Some(integers.as_list_ref())).unwrap();
        let before = (
            builder.values.clone(),
            builder.offsets.clone(),
            builder.validity.clone(),
        );
        assert_eq!(
            builder.push(Some(strings.as_list_ref())),
            Err(ListError::TypeMismatch(TypeMismatch {
                expected: PhysicalType::Int32,
                actual: PhysicalType::String,
            }))
        );
        assert_eq!((builder.values, builder.offsets, builder.validity), before);
    }

    #[test]
    fn empty_all_null_and_populated_decimal_lists_keep_child_metadata() {
        let decimal_type = DecimalType::try_new(8, 2).unwrap();
        let value = Decimal::try_new(12_345, decimal_type).unwrap();
        let child = DecimalArray::try_from_slice(decimal_type, &[Some(value), None]).unwrap();
        let scalar = ListScalar::try_new(child.into()).unwrap();
        let element_type = PhysicalType::Decimal(decimal_type);

        let populated = super::ListArray::try_from_rows(
            element_type.clone(),
            [Some(scalar.as_list_ref()), None],
        )
        .unwrap();
        assert_eq!(populated.element_type(), element_type);
        assert_eq!(
            populated.get(0).unwrap().unwrap().get(0).unwrap(),
            Some(value.into())
        );
        assert_eq!(populated.get(0).unwrap().unwrap().get(1).unwrap(), None);
        assert_eq!(populated.get(1).unwrap(), None);

        let all_null = super::ListArray::try_from_rows(element_type.clone(), [None, None]).unwrap();
        assert_eq!(all_null.element_type(), element_type);
        assert_eq!(all_null.values().physical_type(), element_type);
        assert_eq!(all_null.values().len(), 0);
    }

    #[test]
    fn mixed_decimal_descriptors_fail_before_list_builder_mutation() {
        let expected_type = DecimalType::try_new(8, 2).unwrap();
        let other_type = DecimalType::try_new(8, 3).unwrap();
        let expected = ListScalar::try_new(
            DecimalArray::try_from_slice(
                expected_type,
                &[Some(Decimal::try_new(100, expected_type).unwrap())],
            )
            .unwrap()
            .into(),
        )
        .unwrap();
        let other = ListScalar::try_new(
            DecimalArray::try_from_slice(
                other_type,
                &[Some(Decimal::try_new(100, other_type).unwrap())],
            )
            .unwrap()
            .into(),
        )
        .unwrap();
        let mut builder = ListArrayBuilder::new(PhysicalType::Decimal(expected_type), 2).unwrap();
        builder.push(Some(expected.as_list_ref())).unwrap();
        let before = (
            builder.values.clone(),
            builder.offsets.clone(),
            builder.validity.clone(),
        );
        assert_eq!(
            builder.push(Some(other.as_list_ref())),
            Err(ListError::TypeMismatch(TypeMismatch {
                expected: PhysicalType::Decimal(expected_type),
                actual: PhysicalType::Decimal(other_type),
            }))
        );
        assert_eq!((builder.values, builder.offsets, builder.validity), before);
    }
}
