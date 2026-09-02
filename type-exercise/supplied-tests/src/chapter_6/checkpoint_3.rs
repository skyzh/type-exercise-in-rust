use crate::*;

#[test]
fn scalar_operations_preserve_public_results_and_context() {
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
    assert_eq!(
        clamp
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int16(5), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int64(0), 1),
            ])
            .unwrap_err()
            .to_string(),
        "function `numeric_clamp` failed at row 0: invalid clamp bounds"
    );
}

#[test]
fn mixed_nullable_multiply_delegates_to_the_generic_binary_loop() {
    let expression = build_numeric_binary_expression(
        "numeric_multiply",
        ArithmeticOperator::Multiply,
        PhysicalType::Int16,
        PhysicalType::Int32,
        PhysicalType::Int32,
    );
    let left: ArrayImpl = I16Array::from_slice(&[Some(3), None, Some(-4)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(5), Some(6), None]).into();
    let output = expression
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(15), None, None]
    );
}

#[test]
fn numeric_negation_and_clamp_batch_kernels_are_strict_and_checked() {
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
    assert_eq!(
        clamp
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int16(5), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int64(0), 1),
            ])
            .unwrap_err()
            .to_string(),
        "function `numeric_clamp` failed at row 0: invalid clamp bounds"
    );

    let float_clamp = build_numeric_clamp_expression(
        "numeric_clamp",
        [
            PhysicalType::Float32,
            PhysicalType::Float32,
            PhysicalType::Float32,
        ],
        PhysicalType::Float32,
    );
    assert_eq!(
        float_clamp
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Float32(1.0), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Float32(f32::NAN), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Float32(2.0), 1),
            ])
            .unwrap_err()
            .to_string(),
        "function `numeric_clamp` failed at row 0: invalid clamp bounds"
    );
}

fn numeric_zero(physical_type: &PhysicalType) -> ScalarRefImpl<'static> {
    match physical_type {
        PhysicalType::Int16 => ScalarRefImpl::Int16(0),
        PhysicalType::Int32 => ScalarRefImpl::Int32(0),
        PhysicalType::Int64 => ScalarRefImpl::Int64(0),
        PhysicalType::Float32 => ScalarRefImpl::Float32(0.0),
        PhysicalType::Float64 => ScalarRefImpl::Float64(0.0),
        _ => unreachable!("numeric physical type"),
    }
}

#[test]
fn clamp_selects_every_legal_two_step_promotion_tuple() {
    let numeric_types = [
        DataType::SmallInt,
        DataType::Integer,
        DataType::BigInt,
        DataType::Real,
        DataType::Double,
    ];

    for value in &numeric_types {
        for lower in &numeric_types {
            let Some(pair) = promote_numeric(value, lower) else {
                continue;
            };
            for upper in &numeric_types {
                let Some(output_type) = promote_numeric(&pair, upper) else {
                    continue;
                };
                let input_types = [
                    value.physical_type(),
                    lower.physical_type(),
                    upper.physical_type(),
                ];
                let expression = build_numeric_clamp_expression(
                    "numeric_clamp",
                    input_types.clone(),
                    output_type.physical_type(),
                );
                let output = expression
                    .evaluate(&[
                        ColumnViewImpl::constant(numeric_zero(&input_types[0]), 1),
                        ColumnViewImpl::constant(numeric_zero(&input_types[1]), 1),
                        ColumnViewImpl::constant(numeric_zero(&input_types[2]), 1),
                    ])
                    .unwrap();
                assert_eq!(output.physical_type(), output_type.physical_type());
            }
        }
    }
}
