use std::collections::HashSet;

use crate::{
    ArithmeticOperator, Array, ArrayImpl, ColumnViewImpl, DataType, Expression, ExpressionError,
    F32Array, F64Array, I16Array, I32Array, I64Array, NUMERIC_PROMOTIONS, NumericPromotion,
    PhysicalType, ScalarError, ScalarRefImpl, promote_numeric,
};

use crate::operators::build_numeric_binary_expression;

const EXPECTED_NUMERIC_PROMOTION_MATRIX: &[(DataType, DataType, Option<DataType>)] = &[
    (
        DataType::SmallInt,
        DataType::SmallInt,
        Some(DataType::SmallInt),
    ),
    (
        DataType::SmallInt,
        DataType::Integer,
        Some(DataType::Integer),
    ),
    (DataType::SmallInt, DataType::BigInt, Some(DataType::BigInt)),
    (DataType::SmallInt, DataType::Real, Some(DataType::Real)),
    (DataType::SmallInt, DataType::Double, Some(DataType::Double)),
    (
        DataType::Integer,
        DataType::SmallInt,
        Some(DataType::Integer),
    ),
    (
        DataType::Integer,
        DataType::Integer,
        Some(DataType::Integer),
    ),
    (DataType::Integer, DataType::BigInt, Some(DataType::BigInt)),
    (DataType::Integer, DataType::Real, Some(DataType::Double)),
    (DataType::Integer, DataType::Double, Some(DataType::Double)),
    (DataType::BigInt, DataType::SmallInt, Some(DataType::BigInt)),
    (DataType::BigInt, DataType::Integer, Some(DataType::BigInt)),
    (DataType::BigInt, DataType::BigInt, Some(DataType::BigInt)),
    (DataType::BigInt, DataType::Real, None),
    (DataType::BigInt, DataType::Double, None),
    (DataType::Real, DataType::SmallInt, Some(DataType::Real)),
    (DataType::Real, DataType::Integer, Some(DataType::Double)),
    (DataType::Real, DataType::BigInt, None),
    (DataType::Real, DataType::Real, Some(DataType::Real)),
    (DataType::Real, DataType::Double, Some(DataType::Double)),
    (DataType::Double, DataType::SmallInt, Some(DataType::Double)),
    (DataType::Double, DataType::Integer, Some(DataType::Double)),
    (DataType::Double, DataType::BigInt, None),
    (DataType::Double, DataType::Real, Some(DataType::Double)),
    (DataType::Double, DataType::Double, Some(DataType::Double)),
];

fn assert_promotion_catalog_matches_matrix(catalog: &[NumericPromotion]) {
    let mut expected_keys = HashSet::new();
    for (left, right, _) in EXPECTED_NUMERIC_PROMOTION_MATRIX {
        assert!(
            expected_keys.insert((left.clone(), right.clone())),
            "duplicate expected promotion key: {left:?}/{right:?}"
        );
    }
    assert_eq!(expected_keys.len(), 25);

    let mut catalog_keys = HashSet::new();
    for entry in catalog {
        assert!(
            catalog_keys.insert((entry.left.clone(), entry.right.clone())),
            "duplicate numeric promotion key: {:?}/{:?}",
            entry.left,
            entry.right
        );
    }

    for (left, right, expected) in EXPECTED_NUMERIC_PROMOTION_MATRIX {
        let actual = catalog
            .iter()
            .find(|entry| &entry.left == left && &entry.right == right)
            .map(|entry| entry.output.clone());
        assert_eq!(actual, expected.clone(), "promotion for {left:?}/{right:?}");
    }

    let expected_present = EXPECTED_NUMERIC_PROMOTION_MATRIX
        .iter()
        .filter(|(_, _, output)| output.is_some())
        .count();
    assert_eq!(catalog_keys.len(), expected_present);
}

fn numeric_expression(
    name: &'static str,
    operator: ArithmeticOperator,
    left: &DataType,
    right: &DataType,
) -> Option<Box<dyn Expression>> {
    let output = promote_numeric(left, right)?;
    Some(build_numeric_binary_expression(
        name,
        operator,
        left.physical_type(),
        right.physical_type(),
        output.physical_type(),
    ))
}

#[test]
fn promotion_catalog_matches_canonical_ordered_matrix() {
    assert_promotion_catalog_matches_matrix(NUMERIC_PROMOTIONS);
    for (left, right, expected) in EXPECTED_NUMERIC_PROMOTION_MATRIX {
        assert_eq!(promote_numeric(left, right), expected.clone());
    }
    assert_eq!(
        promote_numeric(
            DataType::Decimal {
                scale: 2,
                precision: 8,
            },
            DataType::Integer,
        ),
        None
    );
}

#[test]
#[should_panic(expected = "duplicate numeric promotion key: SmallInt/Integer")]
fn promotion_catalog_regression_rejects_duplicate_row_substitution() {
    let mut mutated = NUMERIC_PROMOTIONS.to_vec();
    for entry in &mut mutated {
        if entry.left == DataType::SmallInt && entry.right == DataType::BigInt {
            *entry = NumericPromotion {
                left: DataType::SmallInt,
                right: DataType::Integer,
                output: DataType::Integer,
            };
        } else if entry.left == DataType::BigInt && entry.right == DataType::SmallInt {
            *entry = NumericPromotion {
                left: DataType::Integer,
                right: DataType::SmallInt,
                output: DataType::Integer,
            };
        }
    }

    assert_promotion_catalog_matches_matrix(&mutated);
}

#[test]
fn arithmetic_promotes_both_operand_orders_and_rejects_lossy_pairs() {
    for (left_type, right_type, left, right) in [
        (
            DataType::SmallInt,
            DataType::Double,
            ScalarRefImpl::Int16(2),
            ScalarRefImpl::Float64(0.5),
        ),
        (
            DataType::Double,
            DataType::SmallInt,
            ScalarRefImpl::Float64(0.5),
            ScalarRefImpl::Int16(2),
        ),
        (
            DataType::Integer,
            DataType::Double,
            ScalarRefImpl::Int32(2),
            ScalarRefImpl::Float64(0.5),
        ),
        (
            DataType::Double,
            DataType::Integer,
            ScalarRefImpl::Float64(0.5),
            ScalarRefImpl::Int32(2),
        ),
        (
            DataType::Integer,
            DataType::Real,
            ScalarRefImpl::Int32(2),
            ScalarRefImpl::Float32(0.5),
        ),
        (
            DataType::Real,
            DataType::Integer,
            ScalarRefImpl::Float32(0.5),
            ScalarRefImpl::Int32(2),
        ),
    ] {
        let expression = numeric_expression(
            "numeric_add",
            ArithmeticOperator::Add,
            &left_type,
            &right_type,
        )
        .unwrap();
        assert_eq!(expression.output_type(), PhysicalType::Float64);
        let output = expression
            .evaluate(&[
                ColumnViewImpl::constant(left, 1),
                ColumnViewImpl::constant(right, 1),
            ])
            .unwrap();
        assert_eq!(<&F64Array>::try_from(&output).unwrap().get(0), Some(2.5));
    }

    for (left, right) in [
        (DataType::BigInt, DataType::Double),
        (DataType::Double, DataType::BigInt),
    ] {
        assert!(
            numeric_expression(
                "numeric_multiply",
                ArithmeticOperator::Multiply,
                &left,
                &right
            )
            .is_none()
        );
    }
}

#[test]
fn signed_arithmetic_wraps_and_division_reports_the_exact_row() {
    for (name, operator, expected) in [
        ("numeric_add", ArithmeticOperator::Add, 13),
        ("numeric_subtract", ArithmeticOperator::Subtract, 5),
        ("numeric_multiply", ArithmeticOperator::Multiply, 36),
        ("numeric_divide", ArithmeticOperator::Divide, 2),
    ] {
        let expression =
            numeric_expression(name, operator, &DataType::Integer, &DataType::Integer).unwrap();
        let output = expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(9), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 1),
            ])
            .unwrap();
        assert_eq!(
            <&I32Array>::try_from(&output).unwrap().get(0),
            Some(expected),
            "{name}"
        );
    }

    let add = numeric_expression(
        "numeric_add",
        ArithmeticOperator::Add,
        &DataType::SmallInt,
        &DataType::SmallInt,
    )
    .unwrap();
    let added = add
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int16(i16::MAX), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int16(1), 1),
        ])
        .unwrap();
    assert_eq!(
        <&I16Array>::try_from(&added).unwrap().get(0),
        Some(i16::MIN)
    );

    let multiply = numeric_expression(
        "numeric_multiply",
        ArithmeticOperator::Multiply,
        &DataType::BigInt,
        &DataType::BigInt,
    )
    .unwrap();
    let multiplied = multiply
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int64(i64::MAX), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int64(2), 1),
        ])
        .unwrap();
    assert_eq!(<&I64Array>::try_from(&multiplied).unwrap().get(0), Some(-2));

    let divide = numeric_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        &DataType::Integer,
        &DataType::Integer,
    )
    .unwrap();
    let numerators: ArrayImpl = I32Array::from_values(vec![8, 9, 10]).into();
    let divisors: ArrayImpl = I32Array::from_values(vec![2, 0, 5]).into();
    assert_eq!(
        divide.evaluate(&[
            ColumnViewImpl::array(&numerators),
            ColumnViewImpl::array(&divisors)
        ]),
        Err(ExpressionError::ScalarEvaluation {
            function: "numeric_divide",
            row: 1,
            error: ScalarError::DivisionByZero,
        })
    );
    assert_eq!(
        divide.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(i32::MIN), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(-1), 1),
        ]),
        Err(ExpressionError::ScalarEvaluation {
            function: "numeric_divide",
            row: 0,
            error: ScalarError::DivisionOverflow,
        })
    );
}

#[test]
fn nulls_short_circuit_scalar_errors_and_float_division_keeps_ieee_results() {
    let integer_divide = numeric_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        &DataType::Integer,
        &DataType::Integer,
    )
    .unwrap();
    let output = integer_divide
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Int32, 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(0), 2),
        ])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![None, None]
    );

    let float_divide = numeric_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        &DataType::Real,
        &DataType::Real,
    )
    .unwrap();
    for zero in [0.0_f32, -0.0] {
        assert_eq!(
            float_divide.evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Float32(1.0), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Float32(zero), 1),
            ]),
            Err(ExpressionError::ScalarEvaluation {
                function: "numeric_divide",
                row: 0,
                error: ScalarError::DivisionByZero,
            })
        );
    }
    let special = float_divide
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Float32(f32::INFINITY), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Float32(f32::INFINITY), 2),
        ])
        .unwrap();
    assert!(
        <&F32Array>::try_from(&special)
            .unwrap()
            .get(0)
            .unwrap()
            .is_nan()
    );
}
