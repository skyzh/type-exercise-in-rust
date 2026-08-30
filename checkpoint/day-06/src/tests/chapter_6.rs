use std::cell::Cell;

use crate::{
    Array, ArrayImpl, ColumnViewImpl, DataType, I16Array, I32Array, I64Array, PhysicalType,
    ScalarRefImpl, evaluate_unary, promote_numeric, validate_expression_inputs,
};

#[test]
fn scalar_operations_reuse_exactly_one_loop_per_arity() {
    let facade = include_str!("../arithmetic.rs");
    let core = include_str!("../expression.rs");
    assert!(facade.contains("fn neg_number<O: Numeric>(value: O) -> O"));
    assert!(facade.contains("fn clamp_number<O: Numeric>"));
    assert!(core.contains("pub fn evaluate_unary"));
    assert!(core.contains("pub fn auto_vectorize_binary"));
    assert!(core.contains("pub fn try_evaluate_ternary"));
    assert!(!facade.contains("for row in 0.."));

    for adapter in [
        "fn evaluate_numeric_add",
        "fn evaluate_numeric_subtract",
        "fn evaluate_numeric_multiply",
        "fn evaluate_numeric_divide",
        "fn evaluate_numeric_neg",
        "fn evaluate_numeric_clamp",
    ] {
        let body = facade
            .split(adapter)
            .nth(1)
            .unwrap()
            .split("\nfn ")
            .next()
            .unwrap();
        assert!(
            !body.contains("for row"),
            "{adapter} duplicated the row loop"
        );
    }
}

#[test]
fn strict_vectorization_skips_the_scalar_function_for_null_rows() {
    let calls = Cell::new(0);
    let input: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
    let output = evaluate_unary::<i32, i32, _>(ColumnViewImpl::array(&input), |value| {
        calls.set(calls.get() + 1);
        value + 10
    })
    .unwrap();

    assert_eq!(calls.get(), 2);
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(11), None, Some(13)]
    );
}

use crate::arithmetic::{build_numeric_clamp_expression, build_numeric_neg_expression};

#[test]
fn direct_mixed_batch_kernel_is_strict_and_reports_the_failing_row() {
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

#[test]
fn validation_is_arity_then_type_then_length_for_any_arity() {
    let wrong_arity = [ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 3)];
    assert_eq!(
        validate_expression_inputs(&wrong_arity, &[PhysicalType::Int32, PhysicalType::Int32],)
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
