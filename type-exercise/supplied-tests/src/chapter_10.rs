use std::future::Future;
use std::task::{Context, Poll, Waker};

use crate::{
    Array, ArrayImpl, AsyncExpression, AsyncExpressionAdapter, BatchExpression, ColumnViewImpl,
    DataType, Expression, FunctionRegistry, I32Array, PhysicalType, ScalarRefImpl,
    auto_vectorize_binary, evaluate_static,
};

fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn add(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_binary::<i32, i32, i32, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        i32::wrapping_add,
    )
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn shares_the_registry_and_erased_expressions_across_threads() {
    assert_send_sync::<FunctionRegistry>();
    assert_send_sync::<Box<dyn Expression>>();
    let registry = FunctionRegistry::with_builtins();
    let name = std::thread::spawn(move || {
        registry
            .bind("+", &[DataType::Integer, DataType::Integer])
            .unwrap()
            .physical_name()
    })
    .join()
    .unwrap();
    assert_eq!(name, "numeric_add");
}

#[test]
fn static_and_erased_async_boundaries_evaluate_one_complete_batch() {
    let expression = BatchExpression::new(
        "add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        add,
    );
    let left: ArrayImpl = I32Array::from_slice(&[Some(1), None]).into();
    let inputs = [
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
    ];
    let static_output = block_on(evaluate_static(&expression, &inputs)).unwrap();
    assert_eq!(
        I32Array::try_from(static_output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(5), None]
    );

    let adapter = AsyncExpressionAdapter::new(Box::new(expression));
    let erased_output = block_on(adapter.evaluate_async(&inputs)).unwrap();
    assert_eq!(
        I32Array::try_from(erased_output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(5), None]
    );
}

#[test]
fn bound_expressions_forward_the_same_async_semantics() {
    let expression = FunctionRegistry::with_builtins()
        .bind("+", &[DataType::Integer, DataType::Integer])
        .unwrap();
    let left: ArrayImpl = I32Array::from_slice(&[Some(2), None]).into();
    let inputs = [
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
    ];
    let output = block_on(expression.evaluate_async(&inputs)).unwrap();
    assert_eq!(
        I32Array::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(5), None]
    );
}
