use crate::{
    Array, ArrayImpl, ColumnViewImpl, I16Array, I64Array, PhysicalType, ScalarRefImpl,
    validate_expression_inputs,
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

    let values: ArrayImpl = I16Array::from_values(vec![5, 15, 25]).into();
    let clamped = clamp
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
            ColumnViewImpl::constant(ScalarRefImpl::Int64(20), 3),
        ])
        .unwrap();
    assert_eq!(
        <&I64Array>::try_from(&clamped)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(10), Some(15), Some(20)]
    );
    assert!(
        clamp
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int16(5), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int64(0), 1),
            ])
            .is_err()
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
    assert!(add.evaluate(&wrong_type_and_length[..1]).is_err());
    assert!(
        add.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int16(1), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int16(2), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int16(3), 1),
        ])
        .is_err()
    );
    assert!(add.evaluate(&wrong_type_and_length).is_err());

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
    assert!(validate_expression_inputs(&columns, &[const { PhysicalType::Int32 }; 5]).is_err());

    // The fifth of six inputs has the wrong physical type with a valid length:
    // physical-type validation must cover every input, not only the first pair.
    let six_inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
        ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(6), 2),
    ];
    assert!(validate_expression_inputs(&six_inputs, &[const { PhysicalType::Int32 }; 6]).is_err());
}
