use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    Array, ArrayImpl, BindError, BoundExpression, ColumnViewImpl, DataType, Expression,
    FunctionRegistry, I32Array, PhysicalType, ScalarRefImpl, build_builtin_expression,
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

    fn evaluate(&self, _inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        unreachable!("the metadata-only expression is never evaluated")
    }
}

fn expect_bind_error(result: Result<BoundExpression, BindError>) {
    assert!(result.is_err(), "binding unexpectedly succeeded");
}

#[test]
fn checkpoint_1_stores_and_validates_logical_metadata_for_any_arity() {
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
        assert_eq!(expression.output_type(), DataType::Integer);
    }
}

#[test]
fn checkpoint_1_rejects_inconsistent_physical_metadata() {
    expect_bind_error(BoundExpression::new(
        build_builtin_expression("string_concat").unwrap(),
        [DataType::Integer, DataType::Integer],
        DataType::Varchar,
    ));
    expect_bind_error(BoundExpression::new(
        build_builtin_expression("string_concat").unwrap(),
        [DataType::Varchar, DataType::Varchar],
        DataType::Integer,
    ));
}

#[test]
fn checkpoint_1_registers_and_evaluates_a_custom_binary_name() {
    let mut registry = FunctionRegistry::default();
    registry.register_binary("wrapping_add", |left, right| {
        BoundExpression::new(
            build_builtin_expression("i32_add").unwrap(),
            [left, right],
            DataType::Integer,
        )
    });

    let expression = registry
        .bind_binary("wrapping_add", DataType::Integer, DataType::Integer)
        .unwrap();
    let values: ArrayImpl = I32Array::from_values(vec![10, 20]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
        ])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(12), Some(22)]
    );
}

#[test]
fn checkpoint_1_reuses_one_registered_slice_factory() {
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

    registry
        .bind("counted_add", &[DataType::Integer, DataType::Integer])
        .unwrap();
    registry
        .bind("counted_add", &[DataType::Integer, DataType::Integer])
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn checkpoint_1_binary_adapter_checks_arity_before_calling_the_factory() {
    let calls = Arc::new(AtomicUsize::new(0));
    let factory_calls = Arc::clone(&calls);
    let mut registry = FunctionRegistry::default();
    registry.register_binary("binary_probe", move |left, right| {
        factory_calls.fetch_add(1, Ordering::SeqCst);
        BoundExpression::new(
            build_builtin_expression("i32_add").unwrap(),
            [left, right],
            DataType::Integer,
        )
    });

    for inputs in [
        vec![],
        vec![DataType::Integer],
        vec![DataType::Integer; 3],
        vec![DataType::Integer; 5],
    ] {
        expect_bind_error(registry.bind("binary_probe", &inputs));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn checkpoint_1_unknown_names_fail_closed() {
    expect_bind_error(FunctionRegistry::default().bind("missing", &[]));
}
