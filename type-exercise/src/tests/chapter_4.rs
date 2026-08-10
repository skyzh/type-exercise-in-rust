use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    Array, ArrayImpl, BinaryScalarFunction, CheckedBinaryExpression, CheckedBinaryScalarFunction,
    CheckedUnaryScalarFunction, ColumnViewImpl, I32Add, I32Array, PhysicalType, ScalarError,
    ScalarRefImpl, StringArray, UnaryExpression, evaluate_binary,
};

struct StringLengthAdd;

impl BinaryScalarFunction for StringLengthAdd {
    type Left = String;
    type Right = i32;
    type Output = i32;

    fn evaluate(&self, left: &str, right: i32) -> i32 {
        i32::try_from(left.len()).unwrap().wrapping_add(right)
    }
}

struct I32PairLabel;

impl BinaryScalarFunction for I32PairLabel {
    type Left = i32;
    type Right = i32;
    type Output = String;

    fn evaluate(&self, left: i32, right: i32) -> String {
        format!("{left}:{right}")
    }
}

#[test]
fn vectorizes_addition_over_arrays_constants_and_indexed() {
    let left: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let result = evaluate_binary(
        &I32Add,
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
    )
    .unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![Some(12), None, Some(32)]
    );

    let values: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2)]).into();
    let keys = [Some(1), Some(0), None];
    let result = evaluate_binary(
        &I32Add,
        ColumnViewImpl::indexed(&keys, &values).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
    )
    .unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![Some(12), Some(11), None]
    );
}

#[test]
fn propagates_nulls_without_calling_the_scalar_function() {
    let left = ColumnViewImpl::null(PhysicalType::Int32, 2);
    let right = ColumnViewImpl::constant(ScalarRefImpl::Int32(i32::MAX), 2);
    let result = evaluate_binary(&I32Add, left, right).unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![None, None]);
}

#[test]
fn vectorizes_a_borrowed_mixed_family_function() {
    let strings: ArrayImpl = StringArray::from_slice(&[Some("rust"), None, Some("db")]).into();
    let result = evaluate_binary(
        &StringLengthAdd,
        ColumnViewImpl::array(&strings),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
    )
    .unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![Some(6), None, Some(4)]
    );
}

#[test]
fn builds_the_associated_output_array_family() {
    let left: ArrayImpl = I32Array::from_slice(&[Some(4), None]).into();
    let result = evaluate_binary(
        &I32PairLabel,
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
    )
    .unwrap();
    let result = <&StringArray>::try_from(&result).unwrap();
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![Some("4:2"), None]);
}

#[test]
fn addition_uses_explicit_wrapping_overflow() {
    let result = evaluate_binary(
        &I32Add,
        ColumnViewImpl::constant(ScalarRefImpl::Int32(i32::MAX), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
    )
    .unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(result.get(0), Some(i32::MIN));
}

#[test]
fn rejects_input_lengths_before_evaluating_rows() {
    assert!(
        evaluate_binary(
            &I32Add,
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
        )
        .is_err()
    );
}

#[test]
fn rejects_physical_types_before_evaluating_rows() {
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    assert!(
        evaluate_binary(
            &I32Add,
            ColumnViewImpl::array(&strings),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        )
        .is_err()
    );
}

struct CheckedNeg;

impl CheckedUnaryScalarFunction for CheckedNeg {
    type Output = i32;

    fn evaluate(&self, input: ScalarRefImpl<'_>) -> Result<i32, ScalarError> {
        let ScalarRefImpl::Int32(input) = input else {
            unreachable!("the expression validates its physical input")
        };
        Ok(input.wrapping_neg())
    }
}

struct CheckedAdd;

impl CheckedBinaryScalarFunction for CheckedAdd {
    type Output = i32;

    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<i32, ScalarError> {
        let (ScalarRefImpl::Int32(left), ScalarRefImpl::Int32(right)) = (left, right) else {
            unreachable!("the expression validates both physical inputs")
        };
        Ok(left.wrapping_add(right))
    }
}

struct CheckedFailOnSeven;

impl CheckedUnaryScalarFunction for CheckedFailOnSeven {
    type Output = i32;

    fn evaluate(&self, input: ScalarRefImpl<'_>) -> Result<i32, ScalarError> {
        let ScalarRefImpl::Int32(input) = input else {
            unreachable!("the expression validates its physical input")
        };
        if input == 7 {
            return Err(ScalarError::DivisionByZero);
        }
        Ok(input)
    }
}

struct CheckedAddFailOnSecondCall {
    calls: Arc<AtomicUsize>,
}

impl CheckedBinaryScalarFunction for CheckedAddFailOnSecondCall {
    type Output = i32;

    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<i32, ScalarError> {
        let (ScalarRefImpl::Int32(left), ScalarRefImpl::Int32(right)) = (left, right) else {
            unreachable!("the expression validates both physical inputs")
        };
        if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err(ScalarError::DivisionByZero);
        }
        Ok(left.wrapping_add(right))
    }
}

#[test]
fn concrete_unary_and_binary_shells_agree_on_validation_nulls_and_output() {
    let unary = UnaryExpression::new("checked_neg", [PhysicalType::Int32], CheckedNeg);
    let unary_output = unary
        .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 2)])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&unary_output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(-7), Some(-7)]
    );

    let binary = CheckedBinaryExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        CheckedAdd,
    );
    let binary_output = binary
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Int32, 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(i32::MAX), 2),
        ])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&binary_output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![None, None]
    );
}

#[test]
fn shells_reject_extra_or_missing_inputs_before_indexing() {
    let binary = CheckedBinaryExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        CheckedAdd,
    );
    assert!(
        binary
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 1),
            ])
            .is_err()
    );
    assert!(
        binary
            .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1)])
            .is_err()
    );

    let unary = UnaryExpression::new("checked_neg", [PhysicalType::Int32], CheckedNeg);
    assert!(
        unary
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
            ])
            .is_err()
    );
    assert!(unary.evaluate(&[]).is_err());
}

#[test]
fn shells_reject_the_second_input_physical_type() {
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    let binary = CheckedBinaryExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        CheckedAdd,
    );
    assert!(
        binary
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
                ColumnViewImpl::array(&strings),
            ])
            .is_err()
    );
}

#[test]
fn shells_reject_mismatched_input_lengths() {
    let binary = CheckedBinaryExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        CheckedAdd,
    );
    assert!(
        binary
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
            ])
            .is_err()
    );
}

#[test]
fn scalar_errors_propagate_as_batch_errors_not_null_rows() {
    let unary = UnaryExpression::new(
        "checked_fail_on_seven",
        [PhysicalType::Int32],
        CheckedFailOnSeven,
    );
    assert!(
        unary
            .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 1)])
            .is_err()
    );
    assert!(
        unary
            .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1)])
            .is_ok()
    );
}

#[test]
fn binary_scalar_errors_propagate_and_stop_later_rows() {
    let calls = Arc::new(AtomicUsize::new(0));
    let binary = CheckedBinaryExpression::new(
        "checked_add_fail_on_second_call",
        [PhysicalType::Int32, PhysicalType::Int32],
        CheckedAddFailOnSecondCall {
            calls: Arc::clone(&calls),
        },
    );
    assert!(
        binary
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
            ])
            .is_err()
    );
    // The error stops the row loop: the third row is never evaluated.
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
