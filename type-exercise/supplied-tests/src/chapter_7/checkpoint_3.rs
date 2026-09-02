use super::checkpoint_1::i32_values;
use crate::*;

#[test]
fn fallible_division_never_observes_a_null_hidden_zero() {
    let expression = build_numeric_binary_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        PhysicalType::Int32,
        PhysicalType::Int32,
        PhysicalType::Int32,
    );
    let hidden_zero: ArrayImpl = I32Array::from_slice(&[None, Some(2)]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(8), 2),
            ColumnViewImpl::array(&hidden_zero),
        ])
        .unwrap();
    assert_eq!(i32_values(&output), vec![None, Some(4)]);

    let visible_zero: ArrayImpl = I32Array::from_slice(&[None, Some(0)]).into();
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(8), 2),
                ColumnViewImpl::array(&visible_zero),
            ])
            .unwrap_err()
            .to_string(),
        "function `numeric_divide` failed at row 1: division by zero"
    );
}

struct WrappingSubtract;

impl BinaryScalarFunction for WrappingSubtract {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate(&self, left: i32, right: i32) -> i32 {
        left.wrapping_sub(right)
    }
}

#[test]
fn primitive_expression_preserves_operand_order() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let right_values: ArrayImpl = I32Array::from_values(vec![1, 2, 4]).into();
    let expression = PrimitiveBinaryExpression::new("subtract", WrappingSubtract);
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::array(&right_values),
        ])
        .unwrap();
    assert_eq!(i32_values(&output), vec![Some(9), None, Some(26)]);

    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 3),
        ])
        .unwrap();
    assert_eq!(i32_values(&output), vec![Some(7), None, Some(27)]);

    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 3),
            ColumnViewImpl::array(&values),
        ])
        .unwrap();
    assert_eq!(i32_values(&output), vec![Some(-7), None, Some(-27)]);

    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(11), 3),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 3),
        ])
        .unwrap();
    assert_eq!(i32_values(&output), vec![Some(7), Some(7), Some(7)]);
}

#[test]
fn preserves_runtime_type_arity_and_length_errors() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let integers: ArrayImpl = I32Array::from_values(vec![1, 2]).into();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();

    assert_eq!(
        expression
            .evaluate(&[ColumnViewImpl::array(&integers)])
            .unwrap_err()
            .to_string(),
        "input arity mismatch: expected 2, got 1"
    );
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&strings),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
            ])
            .unwrap_err()
            .to_string(),
        "input 0 type mismatch: expected Int32, got String"
    );
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&integers),
                ColumnViewImpl::array(&strings),
            ])
            .unwrap_err()
            .to_string(),
        "input 1 type mismatch: expected Int32, got String"
    );
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&integers),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
            ])
            .unwrap_err()
            .to_string(),
        "input 1 length mismatch: expected 2, got 1"
    );
}
