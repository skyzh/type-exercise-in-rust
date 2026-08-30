use crate::{
    Array, ArrayImpl, ColumnViewImpl, I16Array, I64Array, PhysicalType, ScalarRefImpl,
    validate_expression_inputs,
};
use crate::operators::build_numeric_clamp_expression;

// === Chapter 6 checkpoint 1 ===

#[test]
fn checkpoint_1_validates_arity_then_type_then_length_for_any_arity() {
    let wrong_arity = [ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 3)];
    assert_eq!(
        validate_expression_inputs(
            &wrong_arity,
            &[PhysicalType::Int32, PhysicalType::Int32],
        )
        .unwrap_err()
        .to_string(),
        "input arity mismatch: expected 2, got 1"
    );

    let wrong_type_and_earlier_length = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
        ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(6), 2),
    ];
    assert_eq!(
        validate_expression_inputs(
            &wrong_type_and_earlier_length,
            &[const { PhysicalType::Int32 }; 6],
        )
        .unwrap_err()
        .to_string(),
        "input 4 type mismatch: expected Int32, got String"
    );

    let columns = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 1),
    ];
    assert_eq!(
        validate_expression_inputs(&columns[..4], &[const { PhysicalType::Int32 }; 4]).unwrap(),
        2
    );
    assert_eq!(
        validate_expression_inputs(&columns, &[const { PhysicalType::Int32 }; 5])
            .unwrap_err()
            .to_string(),
        "input 4 length mismatch: expected 2, got 1"
    );
}

// === Chapter 6 checkpoint 2 ===

#[test]
fn checkpoint_2_runs_one_mixed_typed_ternary_loop() {
    let expression = build_numeric_clamp_expression(
        "mixed_clamp",
        [
            PhysicalType::Int16,
            PhysicalType::Int32,
            PhysicalType::Int64,
        ],
        PhysicalType::Int64,
    );
    let values: ArrayImpl = I16Array::from_slice(&[Some(5), None, Some(25)]).into();
    let uppers: ArrayImpl = I64Array::from_values(vec![20, 0, 20]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
            ColumnViewImpl::array(&uppers),
        ])
        .unwrap();
    assert_eq!(
        <&I64Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(10), None, Some(20)]
    );

    let invalid_uppers: ArrayImpl = I64Array::from_values(vec![20, 0]).into();
    assert_eq!(
        expression
            .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int16(5), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 2),
            ColumnViewImpl::array(&invalid_uppers),
            ])
            .unwrap_err()
            .to_string(),
        "function `mixed_clamp` failed at row 1: invalid clamp bounds"
    );
}
