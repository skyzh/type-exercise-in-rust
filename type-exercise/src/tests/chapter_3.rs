use crate::{
    Array, ArrayImpl, ColumnViewImpl, ExpressionError, I32Add, I32Array, PhysicalType,
    ScalarRefImpl, StringArray, TypeMismatch, evaluate_binary,
};

#[test]
fn vectorizes_addition_over_arrays_constants_and_dictionaries() {
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
        ColumnViewImpl::dictionary(&keys, &values).unwrap(),
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
fn rejects_input_lengths_before_evaluating_rows() {
    assert_eq!(
        evaluate_binary(
            &I32Add,
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
        ),
        Err(ExpressionError::InputLengthMismatch { left: 2, right: 3 })
    );
}

#[test]
fn rejects_physical_types_before_evaluating_rows() {
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    assert_eq!(
        evaluate_binary(
            &I32Add,
            ColumnViewImpl::array(&strings),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }))
    );
}
