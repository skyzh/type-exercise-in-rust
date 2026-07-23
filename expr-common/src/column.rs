// Copyright 2022-2026 Alex Chi. Licensed under Apache-2.0.

//! Zero-copy input views for expression evaluation.
//!
//! An expression should care about the logical sequence of nullable values, not whether the
//! sequence is stored as an Arrow-like array, a dictionary encoding, or one repeated constant.
//! [`ColumnView`] is the typed interface used by kernels. [`ColumnViewImpl`] is its type-erased
//! counterpart at the expression boundary.

use thiserror::Error;

use crate::TypeMismatch;
use crate::array::{Array, ArrayImpl, PhysicalType};
use crate::scalar::{Scalar, ScalarRefImpl};

/// A type-erased, borrowed input column.
///
/// The view owns no value buffers. Cloning or copying it is therefore inexpensive.
#[derive(Clone, Copy, Debug)]
pub enum ColumnViewImpl<'a> {
    /// A regular Arrow-like array.
    Array(&'a ArrayImpl),
    /// One scalar repeated `len` times. `physical_type` also types a null constant.
    Constant {
        value: Option<ScalarRefImpl<'a>>,
        physical_type: PhysicalType,
        len: usize,
    },
    /// Nullable row indices projected into a regular dictionary-values array.
    Dictionary {
        indices: &'a [Option<usize>],
        values: &'a ArrayImpl,
    },
}

/// An invalid dictionary encoding supplied to a column view.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error(
    "dictionary key {key} at row {row} is out of bounds for a dictionary of length {dictionary_len}"
)]
pub struct InvalidDictionaryKey {
    pub row: usize,
    pub key: usize,
    pub dictionary_len: usize,
}

impl<'a> ColumnViewImpl<'a> {
    /// Borrow a regular array as a column view.
    pub fn array(array: &'a ArrayImpl) -> Self {
        Self::Array(array)
    }

    /// Repeat a non-null scalar without materializing an array.
    pub fn constant(value: ScalarRefImpl<'a>, len: usize) -> Self {
        Self::Constant {
            physical_type: value.physical_type(),
            value: Some(value),
            len,
        }
    }

    /// Repeat a typed null without materializing an array.
    pub fn null(physical_type: PhysicalType, len: usize) -> Self {
        Self::Constant {
            value: None,
            physical_type,
            len,
        }
    }

    /// Project nullable indices into a dictionary-values array.
    pub fn dictionary(
        indices: &'a [Option<usize>],
        values: &'a ArrayImpl,
    ) -> Result<Self, InvalidDictionaryKey> {
        if let Some((row, key)) = indices
            .iter()
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
        Ok(Self::Dictionary { indices, values })
    }

    /// Number of logical rows exposed by the view.
    pub fn len(&self) -> usize {
        match self {
            Self::Array(array) => array.len(),
            Self::Constant { len, .. } => *len,
            Self::Dictionary { indices, .. } => indices.len(),
        }
    }

    /// Whether this view contains no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Physical value type exposed by this view.
    pub fn physical_type(&self) -> PhysicalType {
        match self {
            Self::Array(array) => array.physical_type(),
            Self::Constant { physical_type, .. } => *physical_type,
            Self::Dictionary { values, .. } => values.physical_type(),
        }
    }
}

impl<'a> From<&'a ArrayImpl> for ColumnViewImpl<'a> {
    fn from(array: &'a ArrayImpl) -> Self {
        Self::array(array)
    }
}

/// Static interface used inside a generated expression's hot loop.
pub trait ColumnAccessor<'a, S: Scalar> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, row: usize) -> Option<S::RefType<'a>>;
}

#[derive(Debug)]
pub struct ArrayColumnView<'a, S: Scalar>(pub &'a S::ArrayType);

impl<'a, S: Scalar> ColumnAccessor<'a, S> for ArrayColumnView<'a, S> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        let array: &'a S::ArrayType = self.0;
        array.get(row)
    }
}

#[derive(Debug)]
pub struct ConstantColumnView<'a, S: Scalar> {
    pub value: Option<S::RefType<'a>>,
    pub len: usize,
}

impl<'a, S: Scalar> ColumnAccessor<'a, S> for ConstantColumnView<'a, S> {
    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn get(&self, _row: usize) -> Option<S::RefType<'a>> {
        self.value
    }
}

#[derive(Debug)]
pub struct DictionaryColumnView<'a, S: Scalar> {
    pub indices: &'a [Option<usize>],
    pub values: &'a S::ArrayType,
}

impl<'a, S: Scalar> ColumnAccessor<'a, S> for DictionaryColumnView<'a, S> {
    #[inline]
    fn len(&self) -> usize {
        self.indices.len()
    }

    #[inline]
    fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        let values: &'a S::ArrayType = self.values;
        self.indices[row].and_then(|key| values.get(key))
    }
}

/// A typed logical column consumed by a scalar kernel.
///
/// `S` connects the logical scalar, its borrowed representation, and its physical array. The
/// generated evaluator matches this enum once, before entering its statically dispatched loop.
#[derive(Debug)]
pub enum ColumnView<'a, S: Scalar> {
    Array(ArrayColumnView<'a, S>),
    Constant(ConstantColumnView<'a, S>),
    Dictionary(DictionaryColumnView<'a, S>),
}

impl<'a, S: Scalar> ColumnView<'a, S> {
    /// Number of logical rows exposed by the view.
    pub fn len(&self) -> usize {
        match self {
            Self::Array(view) => view.len(),
            Self::Constant(view) => view.len(),
            Self::Dictionary(view) => view.len(),
        }
    }

    /// Whether this view contains no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Read one nullable logical value, independent of its physical encoding.
    pub fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        assert!(row < self.len(), "column view row out of bounds");
        match self {
            Self::Array(view) => view.get(row),
            Self::Constant(view) => view.get(row),
            Self::Dictionary(view) => view.get(row),
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
            return Err(TypeMismatch(S::PHYSICAL_TYPE, view.physical_type()));
        }

        match view {
            ColumnViewImpl::Array(array) => Ok(Self::Array(ArrayColumnView(array.try_into()?))),
            ColumnViewImpl::Constant { value, len, .. } => Ok(Self::Constant(ConstantColumnView {
                value: value.map(TryInto::try_into).transpose()?,
                len,
            })),
            ColumnViewImpl::Dictionary { indices, values } => {
                Ok(Self::Dictionary(DictionaryColumnView {
                    indices,
                    values: values.try_into()?,
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::{Array, I32Array, StringArray};

    #[test]
    fn reads_array_constant_and_dictionary_through_one_type() {
        let array = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
        let array_view = ColumnView::<i32>::try_from(ColumnViewImpl::array(&array)).unwrap();
        assert_eq!(array_view.get(0), Some(10));
        assert_eq!(array_view.get(1), None);

        let constant =
            ColumnView::<i32>::try_from(ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 3))
                .unwrap();
        assert_eq!(constant.get(2), Some(7));

        let dictionary_values = StringArray::from_slice(&[Some("red"), Some("green")]).into();
        let indices = [Some(1), None, Some(0), Some(1)];
        let erased = ColumnViewImpl::dictionary(&indices, &dictionary_values).unwrap();
        let dictionary = ColumnView::<String>::try_from(erased).unwrap();
        assert_eq!(dictionary.get(0), Some("green"));
        assert_eq!(dictionary.get(1), None);
        assert_eq!(dictionary.get(2), Some("red"));
    }

    #[test]
    fn rejects_invalid_dictionary_keys() {
        let values = I32Array::from_slice(&[Some(1)]).into();
        let error = ColumnViewImpl::dictionary(&[Some(1)], &values).unwrap_err();
        assert_eq!(
            error,
            InvalidDictionaryKey {
                row: 0,
                key: 1,
                dictionary_len: 1,
            }
        );
    }
}
