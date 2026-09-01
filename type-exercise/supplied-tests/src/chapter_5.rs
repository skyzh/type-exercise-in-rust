use std::collections::HashSet;

use crate::{
    ArithmeticOperator, Array, ArrayImpl, BoolArray, ColumnViewImpl, ComparisonOperator, DataType,
    DecimalType, Expression, F32Array, F64Array, I16Array, I32Array, I64Array, NUMERIC_PROMOTIONS,
    NumericPromotion, PhysicalType, ScalarRefImpl, StringArray, promote_numeric,
};

use crate::{build_numeric_binary_expression, build_numeric_comparison_expression};

#[test]
fn arithmetic_preserves_success_null_and_failure_semantics() {
    let add = build_numeric_binary_expression(
        "numeric_add",
        ArithmeticOperator::Add,
        PhysicalType::Int16,
        PhysicalType::Int32,
        PhysicalType::Int32,
    );
    let left: ArrayImpl = I16Array::from_slice(&[Some(3), None, Some(-4)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(5), Some(6), None]).into();
    let output = add
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(8), None, None]
    );

    let divide = build_numeric_binary_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        PhysicalType::Int32,
        PhysicalType::Int32,
        PhysicalType::Int32,
    );
    let numerators: ArrayImpl = I32Array::from_values(vec![8, 9, 10]).into();
    let divisors: ArrayImpl = I32Array::from_values(vec![2, 0, 5]).into();
    assert_eq!(
        divide
            .evaluate(&[
                ColumnViewImpl::array(&numerators),
                ColumnViewImpl::array(&divisors),
            ])
            .unwrap_err()
            .to_string(),
        "function `numeric_divide` failed at row 1: division by zero"
    );
}

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

#[test]
fn promotion_catalog_matches_canonical_ordered_matrix() {
    assert_promotion_catalog_matches_matrix(NUMERIC_PROMOTIONS);
    for (left, right, expected) in EXPECTED_NUMERIC_PROMOTION_MATRIX {
        assert_eq!(promote_numeric(left, right), expected.clone());
    }
    assert_eq!(
        promote_numeric(
            DataType::Decimal(DecimalType::try_new(8, 2).unwrap()),
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
        let output_type = promote_numeric(&left_type, &right_type).unwrap();
        let expression = build_numeric_binary_expression(
            "numeric_add",
            ArithmeticOperator::Add,
            left_type.physical_type(),
            right_type.physical_type(),
            output_type.physical_type(),
        );
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
        (DataType::BigInt, DataType::Real),
        (DataType::Real, DataType::BigInt),
    ] {
        assert_eq!(promote_numeric(&left, &right), None, "{left:?}/{right:?}");
    }
}

#[test]
fn signed_arithmetic_wraps_and_division_reports_a_batch_error() {
    for (name, operator, expected) in [
        ("numeric_add", ArithmeticOperator::Add, 13),
        ("numeric_subtract", ArithmeticOperator::Subtract, 5),
        ("numeric_multiply", ArithmeticOperator::Multiply, 36),
        ("numeric_divide", ArithmeticOperator::Divide, 2),
    ] {
        let output_type = promote_numeric(&DataType::Integer, &DataType::Integer).unwrap();
        let expression = build_numeric_binary_expression(
            name,
            operator,
            DataType::Integer.physical_type(),
            DataType::Integer.physical_type(),
            output_type.physical_type(),
        );
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

    let output_type = promote_numeric(&DataType::SmallInt, &DataType::SmallInt).unwrap();
    let add = build_numeric_binary_expression(
        "numeric_add",
        ArithmeticOperator::Add,
        DataType::SmallInt.physical_type(),
        DataType::SmallInt.physical_type(),
        output_type.physical_type(),
    );
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

    let output_type = promote_numeric(&DataType::BigInt, &DataType::BigInt).unwrap();
    let multiply = build_numeric_binary_expression(
        "numeric_multiply",
        ArithmeticOperator::Multiply,
        DataType::BigInt.physical_type(),
        DataType::BigInt.physical_type(),
        output_type.physical_type(),
    );
    let multiplied = multiply
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int64(i64::MAX), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int64(2), 1),
        ])
        .unwrap();
    assert_eq!(<&I64Array>::try_from(&multiplied).unwrap().get(0), Some(-2));

    let output_type = promote_numeric(&DataType::Integer, &DataType::Integer).unwrap();
    let divide = build_numeric_binary_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        DataType::Integer.physical_type(),
        DataType::Integer.physical_type(),
        output_type.physical_type(),
    );
    let numerators: ArrayImpl = I32Array::from_values(vec![8, 9, 10]).into();
    let divisors: ArrayImpl = I32Array::from_values(vec![2, 0, 5]).into();
    let division_by_zero = divide
        .evaluate(&[
            ColumnViewImpl::array(&numerators),
            ColumnViewImpl::array(&divisors),
        ])
        .unwrap_err();
    assert!(
        division_by_zero
            .to_string()
            .contains("row 1: division by zero"),
        "{division_by_zero:#}"
    );
    let overflow = divide
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(i32::MIN), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(-1), 1),
        ])
        .unwrap_err();
    assert!(
        overflow
            .to_string()
            .contains("row 0: signed integer division overflow"),
        "{overflow:#}"
    );
}

#[test]
fn nulls_short_circuit_scalar_errors_and_float_division_keeps_ieee_results() {
    let output_type = promote_numeric(&DataType::Integer, &DataType::Integer).unwrap();
    let integer_divide = build_numeric_binary_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        DataType::Integer.physical_type(),
        DataType::Integer.physical_type(),
        output_type.physical_type(),
    );
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

    let output_type = promote_numeric(&DataType::Real, &DataType::Real).unwrap();
    let float_divide = build_numeric_binary_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        DataType::Real.physical_type(),
        DataType::Real.physical_type(),
        output_type.physical_type(),
    );
    for zero in [0.0_f32, -0.0] {
        assert!(
            float_divide
                .evaluate(&[
                    ColumnViewImpl::constant(ScalarRefImpl::Float32(1.0), 1),
                    ColumnViewImpl::constant(ScalarRefImpl::Float32(zero), 1),
                ])
                .is_err()
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

#[test]
fn comparisons_cover_every_operator_and_both_operand_orders() {
    for (left_type, right_type, left, right) in [
        (
            DataType::SmallInt,
            DataType::Double,
            ScalarRefImpl::Int16(2),
            ScalarRefImpl::Float64(2.0),
        ),
        (
            DataType::Double,
            DataType::SmallInt,
            ScalarRefImpl::Float64(5.0),
            ScalarRefImpl::Int16(5),
        ),
        (
            DataType::Integer,
            DataType::Integer,
            ScalarRefImpl::Int32(5),
            ScalarRefImpl::Int32(5),
        ),
    ] {
        let common_type = promote_numeric(&left_type, &right_type).unwrap();
        let equal = build_numeric_comparison_expression(
            "numeric_equal",
            ComparisonOperator::Equal,
            left_type.physical_type(),
            right_type.physical_type(),
            common_type.physical_type(),
        );
        let output = equal
            .evaluate(&[
                ColumnViewImpl::constant(left, 1),
                ColumnViewImpl::constant(right, 1),
            ])
            .unwrap();
        assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(true));
    }

    let (left_type, right_type) = (DataType::Integer, DataType::Integer);
    let cases = [
        (ComparisonOperator::Less, 2, 5, true),
        (ComparisonOperator::Less, 5, 5, false),
        (ComparisonOperator::LessOrEqual, 5, 5, true),
        (ComparisonOperator::Greater, 7, 5, true),
        (ComparisonOperator::Greater, 5, 5, false),
        (ComparisonOperator::GreaterOrEqual, 5, 5, true),
        (ComparisonOperator::Equal, 5, 5, true),
        (ComparisonOperator::Equal, 2, 5, false),
        (ComparisonOperator::NotEqual, 2, 5, true),
        (ComparisonOperator::NotEqual, 5, 5, false),
    ];
    for (operator, left, right, expected) in cases {
        let common_type = promote_numeric(&left_type, &right_type).unwrap();
        let expression = build_numeric_comparison_expression(
            "numeric_compare",
            operator,
            left_type.physical_type(),
            right_type.physical_type(),
            common_type.physical_type(),
        );
        let output = expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(left), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(right), 1),
            ])
            .unwrap();
        assert_eq!(
            <&BoolArray>::try_from(&output).unwrap().get(0),
            Some(expected),
            "{operator:?} {left} vs {right}"
        );
    }
}

#[test]
fn nan_comparisons_are_false_except_not_equal() {
    let (left_type, right_type) = (DataType::Double, DataType::Double);
    for operator in [
        ComparisonOperator::Less,
        ComparisonOperator::LessOrEqual,
        ComparisonOperator::Greater,
        ComparisonOperator::GreaterOrEqual,
        ComparisonOperator::Equal,
    ] {
        let common_type = promote_numeric(&left_type, &right_type).unwrap();
        let expression = build_numeric_comparison_expression(
            "numeric_compare",
            operator,
            left_type.physical_type(),
            right_type.physical_type(),
            common_type.physical_type(),
        );
        let output = expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Float64(f64::NAN), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Float64(1.0), 1),
            ])
            .unwrap();
        assert_eq!(
            <&BoolArray>::try_from(&output).unwrap().get(0),
            Some(false),
            "{operator:?} with NaN"
        );
    }

    let common_type = promote_numeric(&left_type, &right_type).unwrap();
    let not_equal = build_numeric_comparison_expression(
        "numeric_not_equal",
        ComparisonOperator::NotEqual,
        left_type.physical_type(),
        right_type.physical_type(),
        common_type.physical_type(),
    );
    let output = not_equal
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Float64(f64::NAN), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Float64(1.0), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(true));
}

#[test]
fn comparison_nulls_short_circuit_before_the_scalar_call() {
    let common_type = promote_numeric(&DataType::Integer, &DataType::Integer).unwrap();
    let expression = build_numeric_comparison_expression(
        "numeric_less",
        ComparisonOperator::Less,
        DataType::Integer.physical_type(),
        DataType::Integer.physical_type(),
        common_type.physical_type(),
    );
    let output = expression
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Int32, 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 2),
        ])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![None, None]
    );
}

#[test]
fn comparison_rejects_wrong_arity_types_and_lengths_before_rows() {
    let common_type = promote_numeric(&DataType::Integer, &DataType::Integer).unwrap();
    let expression = build_numeric_comparison_expression(
        "numeric_less",
        ComparisonOperator::Less,
        DataType::Integer.physical_type(),
        DataType::Integer.physical_type(),
        common_type.physical_type(),
    );

    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 1),
            ])
            .is_err()
    );
    assert!(
        expression
            .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1)])
            .is_err()
    );

    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
                ColumnViewImpl::array(&strings),
            ])
            .is_err()
    );

    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
            ])
            .is_err()
    );
}
