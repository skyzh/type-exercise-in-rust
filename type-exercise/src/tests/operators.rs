use crate::{
    Array, ArrayImpl, BindError, BoolArray, BoundExpression, ColumnViewImpl, DataType,
    ExpressionError, F32Array, F64Array, FunctionRegistry, I16Array, I32Array, I64Array,
    NUMERIC_PROMOTIONS, PhysicalType, ScalarError, ScalarRefImpl, StringArray, TypeMismatch,
    promote_numeric, validate_expression_inputs,
};

fn expect_bind_error(result: Result<BoundExpression, BindError>) -> BindError {
    match result {
        Ok(_) => panic!("binding unexpectedly succeeded"),
        Err(error) => error,
    }
}

#[test]
fn promotion_catalog_is_explicit_symmetric_and_lossless() {
    assert_eq!(NUMERIC_PROMOTIONS.len(), 21);
    for entry in NUMERIC_PROMOTIONS {
        assert_eq!(promote_numeric(entry.left, entry.right), Some(entry.output));
        assert_eq!(promote_numeric(entry.right, entry.left), Some(entry.output));
    }
    assert_eq!(
        promote_numeric(DataType::Integer, DataType::Real),
        Some(DataType::Double)
    );
    assert_eq!(
        promote_numeric(DataType::Real, DataType::Integer),
        Some(DataType::Double)
    );
    assert_eq!(promote_numeric(DataType::BigInt, DataType::Double), None);
    assert_eq!(promote_numeric(DataType::Double, DataType::BigInt), None);
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
fn arithmetic_promotes_both_operand_orders_and_rejects_lossy_pairs() {
    let registry = FunctionRegistry::with_builtins();
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
        let expression = registry.bind_binary("+", left_type, right_type).unwrap();
        assert_eq!(expression.output_type(), DataType::Double);
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
        assert_eq!(
            expect_bind_error(registry.bind_binary("*", left, right)),
            BindError::UnsupportedArguments {
                name: "*".to_owned(),
                inputs: vec![left, right]
            }
        );
    }
}

#[test]
fn signed_arithmetic_wraps_and_division_reports_the_exact_row() {
    let registry = FunctionRegistry::with_builtins();
    for (name, expected) in [("+", 13), ("-", 5), ("*", 36), ("/", 2)] {
        let expression = registry
            .bind_binary(name, DataType::Integer, DataType::Integer)
            .unwrap();
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

    let add = registry
        .bind_binary("+", DataType::SmallInt, DataType::SmallInt)
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

    let multiply = registry
        .bind_binary("*", DataType::BigInt, DataType::BigInt)
        .unwrap();
    let multiplied = multiply
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int64(i64::MAX), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int64(2), 1),
        ])
        .unwrap();
    assert_eq!(<&I64Array>::try_from(&multiplied).unwrap().get(0), Some(-2));

    let divide = registry
        .bind_binary("/", DataType::Integer, DataType::Integer)
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
    let registry = FunctionRegistry::with_builtins();
    let integer_divide = registry
        .bind_binary("/", DataType::Integer, DataType::Integer)
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

    let float_divide = registry
        .bind_binary("/", DataType::Real, DataType::Real)
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

#[test]
fn comparison_names_and_nan_semantics_are_not_swappable() {
    let registry = FunctionRegistry::with_builtins();
    for (name, physical_name, expected) in [
        ("<", "numeric_less", true),
        ("<=", "numeric_less_or_equal", true),
        (">", "numeric_greater", false),
        (">=", "numeric_greater_or_equal", false),
        ("=", "numeric_equal", false),
        ("!=", "numeric_not_equal", true),
    ] {
        let expression = registry
            .bind_binary(name, DataType::Integer, DataType::BigInt)
            .unwrap();
        assert_eq!(expression.physical_name(), physical_name);
        let output = expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int64(3), 1),
            ])
            .unwrap();
        assert_eq!(
            <&BoolArray>::try_from(&output).unwrap().get(0),
            Some(expected),
            "{name}"
        );
    }

    for (name, expected) in [
        ("<", false),
        ("<=", false),
        (">", false),
        (">=", false),
        ("=", false),
        ("!=", true),
    ] {
        let expression = registry
            .bind_binary(name, DataType::Double, DataType::Double)
            .unwrap();
        let output = expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Float64(f64::NAN), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Float64(1.0), 1),
            ])
            .unwrap();
        assert_eq!(
            <&BoolArray>::try_from(&output).unwrap().get(0),
            Some(expected),
            "{name}"
        );
    }
}

#[test]
fn strings_compare_and_contains_with_strict_nulls() {
    let registry = FunctionRegistry::with_builtins();
    let contains = registry
        .bind_binary("contains", DataType::Char { width: 8 }, DataType::Varchar)
        .unwrap();
    let haystacks: ArrayImpl =
        StringArray::from_slice(&[Some("database"), None, Some("rust")]).into();
    let output = contains
        .evaluate(&[
            ColumnViewImpl::array(&haystacks),
            ColumnViewImpl::constant(ScalarRefImpl::String("base"), 3),
        ])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(true), None, Some(false)]
    );

    let less = registry
        .bind_binary("<", DataType::Varchar, DataType::Char { width: 4 })
        .unwrap();
    let output = less
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::String("alpha"), 1),
            ColumnViewImpl::constant(ScalarRefImpl::String("beta"), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(true));
}

#[test]
fn unary_and_ternary_adapters_are_strict_and_checked() {
    let registry = FunctionRegistry::with_builtins();
    let neg = registry.bind("neg", &[DataType::BigInt]).unwrap();
    let output = neg
        .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int64(i64::MIN), 1)])
        .unwrap();
    assert_eq!(
        <&I64Array>::try_from(&output).unwrap().get(0),
        Some(i64::MIN)
    );

    let clamp = registry
        .bind(
            "clamp",
            &[DataType::SmallInt, DataType::Integer, DataType::BigInt],
        )
        .unwrap();
    assert_eq!(clamp.output_type(), DataType::BigInt);
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
fn boundary_validation_fails_closed_for_two_four_and_five_inputs() {
    let registry = FunctionRegistry::with_builtins();
    assert_eq!(
        expect_bind_error(registry.bind("neg", &[])),
        BindError::InputArityMismatch {
            name: "neg".to_owned(),
            expected: 1,
            actual: 0,
        }
    );
    assert_eq!(
        expect_bind_error(registry.bind("clamp", &[DataType::Integer; 4])),
        BindError::InputArityMismatch {
            name: "clamp".to_owned(),
            expected: 3,
            actual: 4,
        }
    );
    let add = registry
        .bind_binary("+", DataType::SmallInt, DataType::SmallInt)
        .unwrap();
    let wrong_type_and_length = [
        ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 3),
        ColumnViewImpl::constant(ScalarRefImpl::Int16(1), 1),
    ];
    assert_eq!(
        add.evaluate(&wrong_type_and_length),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int16,
            actual: PhysicalType::String
        }))
    );
    assert_eq!(
        add.evaluate(&wrong_type_and_length[..1]),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 1
        })
    );

    let columns = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 1),
    ];
    assert_eq!(
        validate_expression_inputs(&columns[..4], &[PhysicalType::Int32; 4]),
        Ok(2)
    );
    assert_eq!(
        validate_expression_inputs(&columns, &[PhysicalType::Int32; 5]),
        Err(ExpressionError::InputLengthMismatch {
            expected: 2,
            actual: 1,
            input_index: 4
        })
    );
}
