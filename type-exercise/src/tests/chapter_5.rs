use crate::{
    Array, ArrayImpl, BindError, BoundExpression, ColumnViewImpl, DataType, Expression,
    ExpressionError, FunctionRegistry, I32Array, PhysicalType, ScalarRefImpl, StringArray,
    TypeMismatch, build_builtin_expression,
};

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

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        if inputs.len() != self.input_types.len() {
            return Err(ExpressionError::InputArityMismatch {
                expected: self.input_types.len(),
                actual: inputs.len(),
            });
        }
        Ok(I32Array::from_slice(&[]).into())
    }
}

fn expect_bind_error(result: Result<BoundExpression, BindError>) -> BindError {
    match result {
        Ok(_) => panic!("binding unexpectedly succeeded"),
        Err(error) => error,
    }
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

    let values: ArrayImpl = StringArray::from_slice(&[Some("data"), Some("rust")]).into();
    let keys = [Some(0), None, Some(1)];
    let inputs = [
        ColumnViewImpl::dictionary(&keys, &values).unwrap(),
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
    assert_eq!(
        expect_bind_error(registry.bind_binary("missing", DataType::Integer, DataType::Integer,)),
        BindError::UnknownFunction {
            name: "missing".to_owned(),
        }
    );
}

#[test]
fn rejects_unsupported_logical_signatures() {
    let registry = FunctionRegistry::with_builtins();
    assert_eq!(
        expect_bind_error(registry.bind_binary("+", DataType::Varchar, DataType::Integer,)),
        BindError::UnsupportedArguments {
            name: "+".to_owned(),
            inputs: vec![DataType::Varchar, DataType::Integer],
        }
    );
    assert_eq!(
        expect_bind_error(registry.bind_binary("+", DataType::Integer, DataType::Varchar,)),
        BindError::UnsupportedArguments {
            name: "+".to_owned(),
            inputs: vec![DataType::Integer, DataType::Varchar],
        }
    );
    assert_eq!(
        expect_bind_error(registry.bind_binary("concat", DataType::Integer, DataType::Varchar,)),
        BindError::UnsupportedArguments {
            name: "concat".to_owned(),
            inputs: vec![DataType::Integer, DataType::Varchar],
        }
    );
    assert_eq!(
        expect_bind_error(registry.bind_binary("concat", DataType::Varchar, DataType::Integer,)),
        BindError::UnsupportedArguments {
            name: "concat".to_owned(),
            inputs: vec![DataType::Varchar, DataType::Integer],
        }
    );
}

#[test]
fn rejects_a_factory_with_inconsistent_physical_metadata() {
    let input_error = expect_bind_error(BoundExpression::new(
        build_builtin_expression("string_concat").unwrap(),
        [DataType::Integer, DataType::Integer],
        DataType::Varchar,
    ));
    assert_eq!(
        input_error,
        BindError::PhysicalSignatureMismatch {
            name: "string_concat",
            expected_inputs: vec![PhysicalType::Int32, PhysicalType::Int32],
            actual_inputs: vec![PhysicalType::String, PhysicalType::String],
            expected_output: PhysicalType::String,
            actual_output: PhysicalType::String,
        }
    );

    let output_error = expect_bind_error(BoundExpression::new(
        build_builtin_expression("string_concat").unwrap(),
        [DataType::Varchar, DataType::Varchar],
        DataType::Integer,
    ));
    assert_eq!(
        output_error,
        BindError::PhysicalSignatureMismatch {
            name: "string_concat",
            expected_inputs: vec![PhysicalType::String, PhysicalType::String],
            actual_inputs: vec![PhysicalType::String, PhysicalType::String],
            expected_output: PhysicalType::Int32,
            actual_output: PhysicalType::String,
        }
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
    assert_eq!(
        expression.evaluate(&inputs),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }))
    );

    assert_eq!(
        expression.evaluate(&inputs[..1]),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 1,
        })
    );
}

#[test]
fn registers_a_custom_logical_name() {
    let mut registry = FunctionRegistry::default();
    registry.register_binary("wrapping_add", |left, right| {
        if left != DataType::Integer || right != DataType::Integer {
            return Err(BindError::UnsupportedArguments {
                name: "wrapping_add".to_owned(),
                inputs: vec![left, right],
            });
        }
        BoundExpression::new(
            build_builtin_expression("i32_add")
                .ok_or(BindError::MissingPhysicalExpression { name: "i32_add" })?,
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
fn slice_registry_rejects_wrong_arity_before_a_binary_factory_can_index() {
    let registry = FunctionRegistry::with_builtins();
    for inputs in [
        vec![],
        vec![DataType::Integer],
        vec![DataType::Integer; 3],
        vec![DataType::Integer; 5],
    ] {
        assert_eq!(
            expect_bind_error(registry.bind("+", &inputs)),
            BindError::InputArityMismatch {
                name: "+".to_owned(),
                expected: 2,
                actual: inputs.len(),
            }
        );
    }
}
