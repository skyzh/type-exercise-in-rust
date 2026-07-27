// Copyright 2022-2026 Alex Chi. Licensed under Apache-2.0.

//! SIMD-friendly fast paths for strict binary functions over primitive values.

use std::marker::PhantomData;

use anyhow::{Result, anyhow};
use expr_common::TypeMismatch;
use expr_common::array::{ArrayImpl, PrimitiveArray, PrimitiveType};
use expr_common::column::{ColumnView, ColumnViewImpl};
use expr_common::expr::Expression;
use expr_common::scalar::{Scalar, ScalarRefImpl};

use crate::BinaryExpression;

/// A strict binary expression with an all-valid primitive fast path.
///
/// Regular arrays and non-null constants are evaluated without per-row validity checks; output
/// validity is initialized in bulk. Nullable inputs and dictionary views delegate to
/// [`BinaryExpression`].
pub struct PrimitiveBinaryExpression<I1, I2, O, F>
where
    I1: PrimitiveType + Scalar<ArrayType = PrimitiveArray<I1>> + Copy,
    I2: PrimitiveType + Scalar<ArrayType = PrimitiveArray<I2>> + Copy,
    O: PrimitiveType + Scalar<ArrayType = PrimitiveArray<O>>,
    for<'a> I1: Scalar<RefType<'a> = I1>,
    for<'a> I2: Scalar<RefType<'a> = I2>,
    for<'a> O: Scalar<RefType<'a> = O>,
    F: Fn(I1, I2) -> O + Send + Sync,
{
    func: F,
    _phantom: PhantomData<(I1, I2, O)>,
}

impl<I1, I2, O, F> PrimitiveBinaryExpression<I1, I2, O, F>
where
    I1: PrimitiveType + Scalar<ArrayType = PrimitiveArray<I1>> + Copy,
    I2: PrimitiveType + Scalar<ArrayType = PrimitiveArray<I2>> + Copy,
    O: PrimitiveType + Scalar<ArrayType = PrimitiveArray<O>>,
    for<'a> I1: Scalar<RefType<'a> = I1>,
    for<'a> I2: Scalar<RefType<'a> = I2>,
    for<'a> O: Scalar<RefType<'a> = O>,
    for<'a> &'a PrimitiveArray<I1>: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a PrimitiveArray<I2>: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> I1: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> I2: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    F: Fn(I1, I2) -> O + Send + Sync,
    PrimitiveArray<O>: Into<ArrayImpl>,
{
    /// Create an expression around one primitive scalar function.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: PhantomData,
        }
    }

    /// Evaluate two type-erased column views.
    pub fn eval_views<'a>(
        &self,
        left: ColumnViewImpl<'a>,
        right: ColumnViewImpl<'a>,
    ) -> Result<ArrayImpl> {
        let typed_left = ColumnView::<I1>::try_from(left)?;
        let typed_right = ColumnView::<I2>::try_from(right)?;
        if typed_left.len() != typed_right.len() {
            return Err(anyhow!(
                "column length mismatch: expected {}, got {}",
                typed_left.len(),
                typed_right.len(),
            ));
        }

        match (typed_left, typed_right) {
            (ColumnView::Array(left), ColumnView::Array(right)) => {
                if let (Some(left), Some(right)) = (left.0.as_non_null(), right.0.as_non_null()) {
                    let values = left
                        .values()
                        .iter()
                        .copied()
                        .zip(right.values().iter().copied())
                        .map(|(left, right)| (self.func)(left, right))
                        .collect();
                    return Ok(PrimitiveArray::from_values(values).into());
                }
            }
            (ColumnView::Array(left), ColumnView::Constant(right)) => {
                if let (Some(left), Some(right)) = (left.0.as_non_null(), right.value) {
                    let values = left
                        .values()
                        .iter()
                        .copied()
                        .map(|left| (self.func)(left, right))
                        .collect();
                    return Ok(PrimitiveArray::from_values(values).into());
                }
            }
            (ColumnView::Constant(left), ColumnView::Array(right)) => {
                if let (Some(left), Some(right)) = (left.value, right.0.as_non_null()) {
                    let values = right
                        .values()
                        .iter()
                        .copied()
                        .map(|right| (self.func)(left, right))
                        .collect();
                    return Ok(PrimitiveArray::from_values(values).into());
                }
            }
            (ColumnView::Constant(left), ColumnView::Constant(right)) => {
                let len = left.len;
                if let (Some(left_value), Some(right_value)) = (left.value, right.value) {
                    let values = std::iter::repeat_with(|| (self.func)(left_value, right_value))
                        .take(len)
                        .collect();
                    return Ok(PrimitiveArray::from_values(values).into());
                }
            }
            _ => {}
        }

        BinaryExpression::<I1, I2, O, _>::new(&self.func).eval_views(left, right)
    }

    /// Evaluate two regular arrays through the column-view adapter.
    pub fn eval_batch(&self, left: &ArrayImpl, right: &ArrayImpl) -> Result<ArrayImpl> {
        self.eval_views(ColumnViewImpl::array(left), ColumnViewImpl::array(right))
    }
}

impl<I1, I2, O, F> Expression for PrimitiveBinaryExpression<I1, I2, O, F>
where
    I1: PrimitiveType + Scalar<ArrayType = PrimitiveArray<I1>> + Copy,
    I2: PrimitiveType + Scalar<ArrayType = PrimitiveArray<I2>> + Copy,
    O: PrimitiveType + Scalar<ArrayType = PrimitiveArray<O>>,
    for<'a> I1: Scalar<RefType<'a> = I1>,
    for<'a> I2: Scalar<RefType<'a> = I2>,
    for<'a> O: Scalar<RefType<'a> = O>,
    for<'a> &'a PrimitiveArray<I1>: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a PrimitiveArray<I2>: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> I1: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> I2: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    F: Fn(I1, I2) -> O + Send + Sync,
    PrimitiveArray<O>: Into<ArrayImpl>,
{
    fn eval(&self, data: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl> {
        if data.len() != 2 {
            return Err(anyhow!("expected 2 inputs for PrimitiveBinaryExpression"));
        }
        self.eval_views(data[0], data[1])
    }
}

#[cfg(test)]
mod tests {
    use expr_common::array::{Array, I32Array};
    use expr_common::scalar::ScalarRefImpl;

    use super::*;

    #[test]
    fn evaluates_all_valid_arrays() {
        let left: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), Some(3)]).into();
        let right: ArrayImpl = I32Array::from_slice(&[Some(3), Some(2), Some(1)]).into();
        let expression =
            PrimitiveBinaryExpression::<i32, i32, bool, _>::new(|left, right| left <= right);

        let result = expression.eval_batch(&left, &right).unwrap();
        assert_eq!(result.get(0), Some(ScalarRefImpl::Bool(true)));
        assert_eq!(result.get(1), Some(ScalarRefImpl::Bool(true)));
        assert_eq!(result.get(2), Some(ScalarRefImpl::Bool(false)));
    }

    #[test]
    fn falls_back_for_nullable_arrays() {
        let left: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
        let right: ArrayImpl = I32Array::from_slice(&[Some(3), Some(2), Some(1)]).into();
        let expression =
            PrimitiveBinaryExpression::<i32, i32, i32, _>::new(|left, right| left + right);

        let result = expression.eval_batch(&left, &right).unwrap();
        assert_eq!(result.get(0), Some(ScalarRefImpl::Int32(4)));
        assert_eq!(result.get(1), None);
        assert_eq!(result.get(2), Some(ScalarRefImpl::Int32(4)));
    }

    #[test]
    fn evaluates_non_null_constants() {
        let left: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), Some(3)]).into();
        let expression =
            PrimitiveBinaryExpression::<i32, i32, i32, _>::new(|left, right| left + right);

        let result = expression
            .eval_views(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
            )
            .unwrap();
        assert_eq!(result.get(0), Some(ScalarRefImpl::Int32(11)));
        assert_eq!(result.get(2), Some(ScalarRefImpl::Int32(13)));
    }
}
