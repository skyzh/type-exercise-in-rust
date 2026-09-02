use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::*;

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

pub(super) fn expect_bind_error(result: Result<BoundExpression, BindError>) {
    assert!(result.is_err(), "binding unexpectedly succeeded");
}

#[test]
fn rejects_an_unknown_logical_function() {
    let registry = FunctionRegistry::with_builtins();
    expect_bind_error(registry.bind_binary("missing", DataType::Integer, DataType::Integer));
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
