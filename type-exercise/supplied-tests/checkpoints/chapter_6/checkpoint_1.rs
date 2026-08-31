use crate::{ColumnViewImpl, PhysicalType, ScalarRefImpl, validate_expression_inputs};

#[test]
fn checkpoint_1_validation_is_arity_then_type_then_length() {
    let wrong_arity = [ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 3)];
    assert_eq!(
        validate_expression_inputs(&wrong_arity, &[PhysicalType::Int32, PhysicalType::Int32])
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
