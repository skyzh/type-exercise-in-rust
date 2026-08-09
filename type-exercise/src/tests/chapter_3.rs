use crate::{
    Array, ArrayImpl, BinaryScalarFunction, ColumnViewImpl, ExpressionError, I32Add, I32Array,
    PhysicalType, ScalarRefImpl, StringArray, TypeMismatch, evaluate_binary,
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
    assert_eq!(
        evaluate_binary(
            &I32Add,
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
        ),
        Err(ExpressionError::InputLengthMismatch {
            expected: 2,
            actual: 3,
            input_index: 1,
        })
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
