use crate::{
    Array, ColumnViewImpl, ExpressionError, I64Array, PhysicalType, ScalarError, ScalarRefImpl,
    TypeMismatch, validate_expression_inputs,
};

use crate::operators::{build_numeric_clamp_expression, build_numeric_neg_expression};

#[test]
fn unary_and_real_ternary_adapters_are_strict_and_checked() {
    let neg = build_numeric_neg_expression("numeric_neg", PhysicalType::Int64);
    let output = neg
        .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int64(i64::MIN), 1)])
        .unwrap();
    assert_eq!(
        <&I64Array>::try_from(&output).unwrap().get(0),
        Some(i64::MIN)
    );

    let clamp = build_numeric_clamp_expression(
        "numeric_clamp",
        [
            PhysicalType::Int16,
            PhysicalType::Int32,
            PhysicalType::Int64,
        ],
        PhysicalType::Int64,
    );
    let output = clamp
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int16(20), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(0), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int64(10), 2),
        ])
        .unwrap();
    assert_eq!(
        <&I64Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(10), Some(10)]
    );

    let null_output = clamp
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Int16, 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int64(0), 1),
        ])
        .unwrap();
    assert_eq!(<&I64Array>::try_from(&null_output).unwrap().get(0), None);
    assert_eq!(
        clamp.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int16(5), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int64(0), 1),
        ]),
        Err(ExpressionError::ScalarEvaluation {
            function: "numeric_clamp",
            row: 0,
            error: ScalarError::InvalidClampBounds,
        })
    );
}

#[test]
fn validation_is_arity_first_and_works_for_four_or_five_inputs() {
    let add = crate::operators::build_numeric_binary_expression(
        "numeric_add",
        crate::ArithmeticOperator::Add,
        PhysicalType::Int16,
        PhysicalType::Int16,
        PhysicalType::Int16,
    );
    let wrong_type_and_length = [
        ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 3),
        ColumnViewImpl::constant(ScalarRefImpl::Int16(1), 1),
    ];
    assert_eq!(
        add.evaluate(&wrong_type_and_length[..1]),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        add.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int16(1), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int16(2), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int16(3), 1),
        ]),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 3,
        })
    );
    assert_eq!(
        add.evaluate(&wrong_type_and_length),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int16,
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
