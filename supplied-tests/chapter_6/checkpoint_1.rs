use crate::{
    ColumnViewImpl, ExpressionError, PhysicalType, ScalarRefImpl, TypeMismatch,
    validate_expression_inputs,
};

// === Chapter 6 checkpoint 1 ===

#[test]
fn checkpoint_1_validates_arity_then_type_then_length_for_any_arity() {
    let wrong_arity = [ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 3)];
    assert_eq!(
        validate_expression_inputs(
            &wrong_arity,
            &[PhysicalType::Int32, PhysicalType::Int32],
        ),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 1,
        })
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
        ),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }))
    );

    let columns = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 1),
    ];
    assert_eq!(
        validate_expression_inputs(&columns[..4], &[const { PhysicalType::Int32 }; 4]),
        Ok(2)
    );
    assert_eq!(
        validate_expression_inputs(&columns, &[const { PhysicalType::Int32 }; 5]),
        Err(ExpressionError::InputLengthMismatch {
            expected: 2,
            actual: 1,
            input_index: 4,
        })
    );
}
