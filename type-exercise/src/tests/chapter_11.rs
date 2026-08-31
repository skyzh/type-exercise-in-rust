use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    Array, ArrayImpl, BindError, BoolArray, BoundExpression, ColumnViewImpl, DataType, Expression,
    FunctionRegistry, I32Array, PhysicalType, PrimitiveLoop, ScalarRefImpl, StringArray,
    build_builtin_expression,
};

fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

struct MetadataExpression {
    input_types: Vec<PhysicalType>,
}

impl Expression for MetadataExpression {
    fn name(&self) -> &'static str {
        "metadata_only"
    }

    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> PhysicalType {
        PhysicalType::Int32
    }

    fn evaluate(&self, _inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        unreachable!("the metadata-only expression is never evaluated")
    }
}

fn expect_bind_error(result: Result<BoundExpression, BindError>) {
    assert!(result.is_err(), "binding unexpectedly succeeded");
}

#[test]
fn binds_integer_addition_before_execution() {
    let registry = FunctionRegistry::with_builtins();
    let expression = registry
        .bind_binary("+", DataType::Integer, DataType::Integer)
        .unwrap();
    assert_eq!(
        expression.input_types(),
        &[DataType::Integer, DataType::Integer]
    );
    assert_eq!(expression.output_type(), DataType::Integer);
    assert_eq!(expression.physical_name(), "i32_add");

    let left: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let inputs = [
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
    ];
    let result = expression.evaluate(&inputs).unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![Some(12), None, Some(32)]
    );
}

#[test]
fn binds_unary_and_real_ternary_functions() {
    let registry = FunctionRegistry::with_builtins();

    let neg = registry.bind("neg", &[DataType::BigInt]).unwrap();
    assert_eq!(neg.input_types(), &[DataType::BigInt]);
    assert_eq!(neg.output_type(), DataType::BigInt);
    assert_eq!(neg.physical_name(), "numeric_neg");

    let clamp = registry
        .bind(
            "clamp",
            &[DataType::SmallInt, DataType::Integer, DataType::BigInt],
        )
        .unwrap();
    assert_eq!(
        clamp.input_types(),
        &[DataType::SmallInt, DataType::Integer, DataType::BigInt]
    );
    assert_eq!(clamp.output_type(), DataType::BigInt);
    assert_eq!(clamp.physical_name(), "numeric_clamp");
}

#[test]
fn preserves_distinct_logical_string_types() {
    let registry = FunctionRegistry::with_builtins();
    let char = DataType::Char { width: 4 };
    for (left, right) in [
        (DataType::Varchar, DataType::Varchar),
        (DataType::Varchar, char.clone()),
        (char.clone(), DataType::Varchar),
        (char.clone(), char.clone()),
    ] {
        let expression = registry
            .bind_binary("concat", left.clone(), right.clone())
            .unwrap();
        assert_eq!(expression.input_types(), &[left, right]);
        assert_eq!(expression.output_type(), DataType::Varchar);
        assert_eq!(expression.physical_name(), "string_concat");
    }

    let expression = registry
        .bind_binary("concat", char, DataType::Varchar)
        .unwrap();

    let values: ArrayImpl = StringArray::from_slice(&[Some("data"), None, Some("rust")]).into();
    let keys = [0, 1, 2];
    let inputs = [
        ColumnViewImpl::indexed(&keys, &values).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::String("base"), 3),
    ];
    let result = expression.evaluate(&inputs).unwrap();
    let result = <&StringArray>::try_from(&result).unwrap();
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![Some("database"), None, Some("rustbase")]
    );
}

#[test]
fn rejects_an_unknown_logical_function() {
    let registry = FunctionRegistry::with_builtins();
    expect_bind_error(registry.bind_binary("missing", DataType::Integer, DataType::Integer));
}

#[test]
fn rejects_unsupported_logical_signatures() {
    let registry = FunctionRegistry::with_builtins();
    for (name, left, right) in [
        ("+", DataType::Varchar, DataType::Integer),
        ("+", DataType::Integer, DataType::Varchar),
        ("concat", DataType::Integer, DataType::Varchar),
        ("concat", DataType::Varchar, DataType::Integer),
    ] {
        expect_bind_error(registry.bind_binary(name, left, right));
    }
}

#[test]
fn rejects_a_factory_with_inconsistent_physical_metadata() {
    assert!(
        BoundExpression::new(
            build_builtin_expression("string_concat").unwrap(),
            [DataType::Integer, DataType::Integer],
            DataType::Varchar,
        )
        .is_err()
    );

    assert!(
        BoundExpression::new(
            build_builtin_expression("string_concat").unwrap(),
            [DataType::Varchar, DataType::Varchar],
            DataType::Integer,
        )
        .is_err()
    );
}

#[test]
fn preserves_checked_execution_errors_after_binding() {
    let registry = FunctionRegistry::with_builtins();
    let expression = registry
        .bind_binary("+", DataType::Integer, DataType::Integer)
        .unwrap();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    let inputs = [
        ColumnViewImpl::array(&strings),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
    ];
    assert!(expression.evaluate(&inputs).is_err());
    assert!(expression.evaluate(&inputs[..1]).is_err());
}

#[test]
fn registers_a_custom_logical_name() {
    let mut registry = FunctionRegistry::default();
    registry.register_binary("wrapping_add", |left, right| {
        assert_eq!((&left, &right), (&DataType::Integer, &DataType::Integer));
        BoundExpression::new(
            build_builtin_expression("i32_add").expect("i32_add is a builtin"),
            [left, right],
            DataType::Integer,
        )
    });

    let expression = registry
        .bind_binary("wrapping_add", DataType::Integer, DataType::Integer)
        .unwrap();
    assert_eq!(expression.physical_name(), "i32_add");
}

#[test]
fn stores_and_validates_slice_metadata_for_any_arity() {
    for input_types in [
        vec![],
        vec![DataType::Integer],
        vec![DataType::Integer; 3],
        vec![DataType::Integer; 5],
    ] {
        let expression = BoundExpression::new(
            Box::new(MetadataExpression {
                input_types: vec![PhysicalType::Int32; input_types.len()],
            }),
            input_types.clone(),
            DataType::Integer,
        )
        .unwrap();
        assert_eq!(expression.input_types(), input_types);
    }
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
            Some(expected)
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
            Some(expected)
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
fn public_string_and_boolean_comparisons_reject_wrong_runtime_arity() {
    let registry = FunctionRegistry::with_builtins();
    let expressions = [
        registry
            .bind_binary("contains", DataType::Varchar, DataType::Varchar)
            .unwrap(),
        registry
            .bind_binary("=", DataType::Boolean, DataType::Boolean)
            .unwrap(),
    ];
    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
    ];

    for expression in expressions {
        for (actual, expected) in [
            (0, "input arity mismatch: expected 2, got 0"),
            (1, "input arity mismatch: expected 2, got 1"),
            (3, "input arity mismatch: expected 2, got 3"),
        ] {
            assert_eq!(
                expression
                    .evaluate(&inputs[..actual])
                    .unwrap_err()
                    .to_string(),
                expected
            );
        }
    }
}

#[test]
fn slice_registry_rejects_wrong_arity_before_a_binary_factory_can_index() {
    let registry = FunctionRegistry::with_builtins();
    for inputs in [
        vec![],
        vec![DataType::Integer],
        vec![DataType::Integer; 3],
        vec![DataType::Integer; 5],
    ] {
        expect_bind_error(registry.bind("+", &inputs));
    }
}

#[test]
fn binds_boolean_and_or_not_with_correct_arity_and_metadata() {
    let registry = FunctionRegistry::with_builtins();

    let and = registry
        .bind_binary("boolean_and", DataType::Boolean, DataType::Boolean)
        .unwrap();
    assert_eq!(and.input_types(), &[DataType::Boolean, DataType::Boolean]);
    assert_eq!(and.output_type(), DataType::Boolean);
    assert_eq!(and.physical_name(), "boolean_and");

    let or = registry
        .bind_binary("boolean_or", DataType::Boolean, DataType::Boolean)
        .unwrap();
    assert_eq!(or.physical_name(), "boolean_or");

    let not = registry.bind("boolean_not", &[DataType::Boolean]).unwrap();
    assert_eq!(not.input_types(), &[DataType::Boolean]);
    assert_eq!(not.output_type(), DataType::Boolean);
    assert_eq!(not.physical_name(), "boolean_not");

    // One-input NOT must bind; two-input AND/OR and wrong-arity shapes fail closed.
    expect_bind_error(registry.bind("boolean_not", &[DataType::Boolean, DataType::Boolean]));
    expect_bind_error(registry.bind("boolean_and", &[DataType::Boolean]));
    expect_bind_error(registry.bind("boolean_and", &[DataType::Integer, DataType::Boolean]));
    expect_bind_error(registry.bind_binary("boolean_not", DataType::Boolean, DataType::Boolean));
}

#[test]
fn bound_boolean_expressions_evaluate_with_sql_semantics() {
    let registry = FunctionRegistry::with_builtins();
    let and = registry
        .bind_binary("boolean_and", DataType::Boolean, DataType::Boolean)
        .unwrap();
    let left: ArrayImpl = BoolArray::from_slice(&[Some(true), None]).into();
    let right: ArrayImpl = BoolArray::from_slice(&[Some(false), Some(false)]).into();
    let output = and
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(false)]
    );

    let not = registry.bind("boolean_not", &[DataType::Boolean]).unwrap();
    let output = not.evaluate(&[ColumnViewImpl::array(&left)]).unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), None]
    );
}

#[test]
fn bound_arithmetic_operators_keep_distinct_semantics() {
    let registry = FunctionRegistry::with_builtins();
    for (name, physical_name, expected) in [
        ("+", "i32_add", 13),
        ("-", "numeric_subtract", 5),
        ("*", "numeric_multiply", 36),
        ("/", "numeric_divide", 2),
    ] {
        let expression = registry
            .bind_binary(name, DataType::Integer, DataType::Integer)
            .unwrap();
        assert_eq!(expression.physical_name(), physical_name, "logical {name}");
        let output = expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(9), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 1),
            ])
            .unwrap();
        assert_eq!(
            <&I32Array>::try_from(&output).unwrap().get(0),
            Some(expected),
            "logical {name}"
        );
    }
}

#[test]
fn binding_rejects_lossy_promotions_for_arithmetic_and_comparisons() {
    let registry = FunctionRegistry::with_builtins();
    let pairs = [
        (DataType::BigInt, DataType::Double),
        (DataType::Double, DataType::BigInt),
        (DataType::BigInt, DataType::Real),
        (DataType::Real, DataType::BigInt),
    ];
    for operator in ["+", "<"] {
        for (left, right) in &pairs {
            expect_bind_error(registry.bind_binary(operator, left.clone(), right.clone()));
        }
    }

    // Evaluation-safety control: the same registry still binds and evaluates a
    // valid widened pair after the lossy rejections above.
    let bound = registry
        .bind_binary("<", DataType::Integer, DataType::BigInt)
        .unwrap();
    let output = bound
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int64(3), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(true));
}

#[test]
fn one_shared_fn_factory_can_be_bound_repeatedly() {
    let calls = Arc::new(AtomicUsize::new(0));
    let factory_calls = Arc::clone(&calls);
    let mut registry = FunctionRegistry::default();
    registry.register("counted_add", move |inputs| {
        factory_calls.fetch_add(1, Ordering::SeqCst);
        let [left, right] = inputs else {
            return Err(BindError::InputArityMismatch {
                name: "counted_add".to_owned(),
                expected: 2,
                actual: inputs.len(),
            });
        };
        BoundExpression::new(
            build_builtin_expression("i32_add").unwrap(),
            [left.clone(), right.clone()],
            DataType::Integer,
        )
    });

    let first = registry
        .bind("counted_add", &[DataType::Integer, DataType::Integer])
        .unwrap();
    let second = registry
        .bind("counted_add", &[DataType::Integer, DataType::Integer])
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(first.physical_name(), "i32_add");
    assert_eq!(second.physical_name(), "i32_add");
}

#[test]
fn keeps_binding_and_non_primitive_catalog_entries_working() {
    let add = FunctionRegistry::with_builtins()
        .bind_binary("+", DataType::Integer, DataType::Integer)
        .unwrap();
    let integers: ArrayImpl = I32Array::from_values(vec![1, 2]).into();
    let (output, selected) = add
        .evaluate_with_loop(&[
            ColumnViewImpl::array(&integers),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 2),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::ArrayConstant);
    assert_eq!(i32_values(&output), vec![Some(6), Some(7)]);

    let concat = build_builtin_expression("string_concat").unwrap();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("data"), None]).into();
    let (output, selected) = concat
        .evaluate_with_loop(&[
            ColumnViewImpl::array(&strings),
            ColumnViewImpl::constant(ScalarRefImpl::String("base"), 2),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::General);
    let output = <&StringArray>::try_from(&output).unwrap();
    assert_eq!(
        output.iter().collect::<Vec<_>>(),
        vec![Some("database"), None]
    );
}
