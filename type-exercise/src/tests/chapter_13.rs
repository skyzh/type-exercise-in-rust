use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::{
    Array, ArrayImpl, BinaryExpression, BoundExpression, ColumnViewImpl, DataType, Expression,
    FunctionRegistry, I32Add, I32Array, PrimitiveBinaryExpression, ScalarRefImpl, StringArray,
    build_builtin_expression,
};

fn assert_send_sync<T: Send + Sync>() {}

fn evaluate_reborrowed<'call, 'data: 'call>(
    expression: &'call dyn Expression,
    inputs: &'call [ColumnViewImpl<'data>],
) -> anyhow::Result<ArrayImpl> {
    let shortened: &'call [ColumnViewImpl<'call>] = inputs;
    expression.evaluate(shortened)
}

#[test]
fn opaque_iterator_preserves_empty_and_nullable_integer_rows() {
    let empty = I32Array::from_slice(&[]);
    assert_eq!(empty.iter().next(), None);

    let nullable = I32Array::from_slice(&[Some(1), None, Some(3)]);
    assert_eq!(
        nullable.iter().collect::<Vec<_>>(),
        vec![Some(1), None, Some(3)]
    );
}

#[test]
fn opaque_iterator_keeps_string_items_borrowed_from_the_array() {
    let strings = StringArray::from_slice(&[Some("rust"), None, Some("database")]);
    let values: Vec<Option<&str>> = strings.iter().collect();
    assert_eq!(values, vec![Some("rust"), None, Some("database")]);
}

#[test]
fn expression_trait_objects_upcast_directly_for_checked_recovery() {
    let add = build_builtin_expression("i32_add").unwrap();
    let erased: &dyn Any = add.as_ref();
    assert!(erased.downcast_ref::<BinaryExpression>().is_none());
    assert!(
        erased
            .downcast_ref::<PrimitiveBinaryExpression<I32Add>>()
            .is_some()
    );

    let concat = build_builtin_expression("string_concat").unwrap();
    let erased: &dyn Any = concat.as_ref();
    assert!(erased.downcast_ref::<BinaryExpression>().is_some());
}

#[test]
fn erased_expressions_are_safe_to_share_with_worker_threads() {
    assert_send_sync::<Box<dyn Expression>>();

    let expression: Arc<dyn Expression> = Arc::from(build_builtin_expression("i32_add").unwrap());
    let worker_expression = Arc::clone(&expression);
    let output = thread::spawn(move || {
        let values: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
        worker_expression
            .evaluate(&[
                ColumnViewImpl::array(&values),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
            ])
            .unwrap()
    })
    .join()
    .unwrap();

    let output = <&I32Array>::try_from(&output).unwrap();
    assert_eq!(
        output.iter().collect::<Vec<_>>(),
        vec![Some(11), None, Some(13)]
    );
}

#[test]
fn logical_factories_can_capture_thread_safe_shared_state() {
    assert_send_sync::<FunctionRegistry>();

    let calls = Arc::new(AtomicUsize::new(0));
    let factory_calls = Arc::clone(&calls);
    let mut registry = FunctionRegistry::default();
    registry.register_binary("counted_add", move |left, right| {
        factory_calls.fetch_add(1, Ordering::SeqCst);
        BoundExpression::new(
            build_builtin_expression("i32_add").unwrap(),
            [left, right],
            DataType::Integer,
        )
    });

    let registry = Arc::new(registry);
    let worker_registry = Arc::clone(&registry);
    let expression = thread::spawn(move || {
        worker_registry.bind_binary("counted_add", DataType::Integer, DataType::Integer)
    })
    .join()
    .unwrap()
    .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(expression.physical_name(), "i32_add");
}

#[test]
fn erased_column_views_are_covariant_at_the_expression_boundary() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(7), None, Some(11)]).into();
    let indices = [2, 0, 1];
    let inputs = [
        ColumnViewImpl::indexed(&indices, &values).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
    ];
    let expression = build_builtin_expression("i32_add").unwrap();
    let output = evaluate_reborrowed(expression.as_ref(), &inputs).unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(16), Some(12), None]
    );
}
