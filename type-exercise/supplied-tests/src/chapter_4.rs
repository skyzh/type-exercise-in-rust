use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    Array, ArrayBuilder, ArrayImpl, BatchExpression, BinaryScalarFunction, ColumnView,
    ColumnViewImpl, I32Add, I32Array, PhysicalType, ScalarRefImpl, StringArray, evaluate_binary,
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

fn assert_copyable_output<F: BinaryScalarFunction>() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<F::Output>();
}

#[test]
fn scalar_adapter_output_is_copyable_and_fixed_width() {
    assert_copyable_output::<I32Add>();
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

    let values: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), None]).into();
    let keys = [1, 0, 2];
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
    let error = evaluate_binary(
        &I32Add,
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
    );
    assert_eq!(
        error.unwrap_err().to_string(),
        "input 1 length mismatch: expected 2, got 3"
    );
}

#[test]
fn rejects_physical_types_before_evaluating_rows() {
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    let left_error = evaluate_binary(
        &I32Add,
        ColumnViewImpl::array(&strings),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
    )
    .unwrap_err();
    assert_eq!(
        left_error.to_string(),
        "input 0 type mismatch: expected Int32, got String"
    );

    let right_error = evaluate_binary(
        &I32Add,
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::array(&strings),
    )
    .unwrap_err();
    assert_eq!(
        right_error.to_string(),
        "input 1 type mismatch: expected Int32, got String"
    );
}

fn checked_neg_batch(
    _expression: &BatchExpression<1>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    let input = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(input.len());
    for row in 0..input.len() {
        output.push(input.get(row).map(i32::wrapping_neg));
    }
    Ok(output.finish().into())
}

fn checked_add_batch(
    _expression: &BatchExpression<2>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let right = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        output.push(
            left.get(row)
                .zip(right.get(row))
                .map(|(left, right)| left.wrapping_add(right)),
        );
    }
    Ok(output.finish().into())
}

fn validation_probe_unary(
    _expression: &BatchExpression<1>,
    _inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    Ok(I32Array::from_slice(&[]).into())
}

fn validation_probe_binary(
    _expression: &BatchExpression<2>,
    _inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    Ok(I32Array::from_slice(&[]).into())
}

fn checked_fail_on_seven_batch(
    _expression: &BatchExpression<1>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    let input = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(input.len());
    for row in 0..input.len() {
        let value = input.get(row);
        if value == Some(7) {
            anyhow::bail!("function `checked_fail_on_seven` failed at row {row}: division by zero");
        }
        output.push(value);
    }
    Ok(output.finish().into())
}

static BATCH_CALLS: AtomicUsize = AtomicUsize::new(0);

fn checked_add_fail_on_second_call_batch(
    _expression: &BatchExpression<2>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let right = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => {
                if BATCH_CALLS.fetch_add(1, Ordering::SeqCst) == 1 {
                    anyhow::bail!(
                        "function `checked_add_fail_on_second_call` failed at row {row}: division by zero"
                    );
                }
                Some(left.wrapping_add(right))
            }
            _ => None,
        };
        output.push(value);
    }
    Ok(output.finish().into())
}

fn checked_string_length_add_batch(
    _expression: &BatchExpression<2>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<String>::try_from(inputs[0].clone())?;
    let right = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        output.push(
            left.get(row)
                .zip(right.get(row))
                .map(|(left, right)| i32::try_from(left.len()).unwrap().wrapping_add(right)),
        );
    }
    Ok(output.finish().into())
}

#[test]
fn batch_kernel_receives_typed_borrowed_inputs() {
    let expression = BatchExpression::new(
        "checked_string_length_add",
        [PhysicalType::String, PhysicalType::Int32],
        PhysicalType::Int32,
        checked_string_length_add_batch,
    );
    let strings: ArrayImpl = StringArray::from_slice(&[Some("typed"), None]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&strings),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
        ])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(7), None]
    );
}

#[test]
fn fixed_arity_batches_agree_on_validation_nulls_and_output() {
    let unary = BatchExpression::new(
        "checked_neg",
        [PhysicalType::Int32],
        PhysicalType::Int32,
        checked_neg_batch,
    );
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

    let binary = BatchExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        checked_add_batch,
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
fn batch_expressions_reject_extra_or_missing_inputs_before_indexing() {
    let binary = BatchExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        validation_probe_binary,
    );
    let extra = binary.evaluate(&[
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 1),
    ]);
    assert_eq!(
        extra.unwrap_err().to_string(),
        "input arity mismatch: expected 2, got 3"
    );
    let missing = binary.evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1)]);
    assert_eq!(
        missing.unwrap_err().to_string(),
        "input arity mismatch: expected 2, got 1"
    );

    let unary = BatchExpression::new(
        "checked_neg",
        [PhysicalType::Int32],
        PhysicalType::Int32,
        validation_probe_unary,
    );
    let extra = unary.evaluate(&[
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
    ]);
    assert_eq!(
        extra.unwrap_err().to_string(),
        "input arity mismatch: expected 1, got 2"
    );
    assert_eq!(
        unary.evaluate(&[]).unwrap_err().to_string(),
        "input arity mismatch: expected 1, got 0"
    );
}

#[test]
fn batch_expressions_reject_the_second_input_physical_type() {
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong"), Some("still wrong")]).into();
    let binary = BatchExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        validation_probe_binary,
    );
    let error = binary.evaluate(&[
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::array(&strings),
    ]);
    assert_eq!(
        error.unwrap_err().to_string(),
        "input 1 type mismatch: expected Int32, got String"
    );
}

#[test]
fn batch_expressions_reject_mismatched_input_lengths() {
    let binary = BatchExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        validation_probe_binary,
    );
    let error = binary.evaluate(&[
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
    ]);
    assert_eq!(
        error.unwrap_err().to_string(),
        "input 1 length mismatch: expected 2, got 3"
    );
}

#[test]
fn scalar_errors_propagate_as_batch_errors_not_null_rows() {
    let unary = BatchExpression::new(
        "checked_fail_on_seven",
        [PhysicalType::Int32],
        PhysicalType::Int32,
        checked_fail_on_seven_batch,
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
    BATCH_CALLS.store(0, Ordering::SeqCst);
    let binary = BatchExpression::new(
        "checked_add_fail_on_second_call",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        checked_add_fail_on_second_call_batch,
    );
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    assert!(
        binary
            .evaluate(&[
                ColumnViewImpl::array(&strings),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
            ])
            .is_err()
    );
    assert_eq!(BATCH_CALLS.load(Ordering::SeqCst), 0);

    let error = binary
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
        ])
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "function `checked_add_fail_on_second_call` failed at row 1: division by zero"
    );
    // The error stops the row loop: the third row is never evaluated.
    assert_eq!(BATCH_CALLS.load(Ordering::SeqCst), 2);
}
