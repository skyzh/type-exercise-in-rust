use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use crate::{
    ArithmeticOperator, Array, ArrayImpl, AsyncExpression, AsyncExpressionAdapter, BatchFuture,
    BoundExpression, ColumnViewImpl, DataType, Expression, ExpressionError, FunctionRegistry,
    I32Array, PhysicalType, ScalarError, ScalarRefImpl, StringArray, TypeMismatch,
    build_builtin_expression, build_numeric_binary_expression, evaluate_static,
};

fn poll_ready<F: Future + ?Sized>(mut future: Pin<&mut F>) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the no-I/O batch future must complete on its first poll"),
    }
}

fn assert_send<T: Send>(_: &T) {}

fn assert_send_sync<T: Send + Sync>() {}

fn borrowed_static_future<'a>(
    expression: &'a dyn Expression,
    inputs: &'a [ColumnViewImpl<'a>],
) -> impl Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a {
    evaluate_static(expression, inputs)
}

fn borrowed_erased_future<'a>(
    expression: &'a dyn AsyncExpression,
    inputs: &'a [ColumnViewImpl<'a>],
) -> BatchFuture<'a> {
    expression.evaluate_async(inputs)
}

#[test]
fn static_future_preserves_the_synchronous_batch_result() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let keys = [2, 0, 1];
    let inputs = [
        ColumnViewImpl::indexed(&keys, &values).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
    ];
    let expression = build_builtin_expression("i32_add").unwrap();
    let expected = expression.evaluate(&inputs).unwrap();

    let mut future = std::pin::pin!(evaluate_static(expression.as_ref(), &inputs));
    assert_send(&future);
    assert_eq!(poll_ready(future.as_mut()).unwrap(), expected);
}

#[test]
fn erased_future_matches_the_static_future() {
    assert_send_sync::<Box<dyn AsyncExpression>>();

    let left: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let inputs = [
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 3),
    ];

    let static_expression = build_builtin_expression("i32_add").unwrap();
    let static_output = {
        let mut future =
            std::pin::pin!(borrowed_static_future(static_expression.as_ref(), &inputs,));
        poll_ready(future.as_mut()).unwrap()
    };

    let expression: Box<dyn AsyncExpression> = Box::new(AsyncExpressionAdapter::new(
        build_builtin_expression("i32_add").unwrap(),
    ));
    let mut future = borrowed_erased_future(expression.as_ref(), &inputs);
    assert_send(&future);
    assert_eq!(poll_ready(future.as_mut()).unwrap(), static_output);
}

#[test]
fn bound_expression_forwards_the_same_typed_result() {
    let expression = FunctionRegistry::with_builtins()
        .bind_binary("concat", DataType::Varchar, DataType::Varchar)
        .unwrap();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("data"), None, Some("rust")]).into();
    let inputs = [
        ColumnViewImpl::array(&strings),
        ColumnViewImpl::constant(ScalarRefImpl::String("base"), 3),
    ];
    let expected = expression.evaluate(&inputs).unwrap();

    let mut future = expression.evaluate_async(&inputs);
    assert_eq!(poll_ready(future.as_mut()).unwrap(), expected);
}

struct CountingExpression {
    calls: Arc<AtomicUsize>,
    inner: Box<dyn Expression>,
}

impl Expression for CountingExpression {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn input_types(&self) -> &[PhysicalType] {
        self.inner.input_types()
    }

    fn output_type(&self) -> PhysicalType {
        self.inner.output_type()
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.evaluate(inputs)
    }
}

#[test]
fn static_future_invokes_synchronous_evaluation_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let expression = CountingExpression {
        calls: Arc::clone(&calls),
        inner: build_builtin_expression("i32_add").unwrap(),
    };
    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
    ];

    let mut future = std::pin::pin!(evaluate_static(&expression, &inputs));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let output = poll_ready(future.as_mut()).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(7), Some(7)]
    );
}

#[test]
fn erased_future_invokes_synchronous_evaluation_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let expression = AsyncExpressionAdapter::new(Box::new(CountingExpression {
        calls: Arc::clone(&calls),
        inner: build_builtin_expression("i32_add").unwrap(),
    }));
    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
    ];

    let mut future = expression.evaluate_async(&inputs);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let output = poll_ready(future.as_mut()).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(7), Some(7)]
    );
}

#[test]
fn bound_future_invokes_synchronous_evaluation_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let expression = BoundExpression::new(
        Box::new(CountingExpression {
            calls: Arc::clone(&calls),
            inner: build_builtin_expression("i32_add").unwrap(),
        }),
        [DataType::Integer, DataType::Integer],
        DataType::Integer,
    )
    .unwrap();
    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
    ];

    let mut future = expression.evaluate_async(&inputs);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let output = poll_ready(future.as_mut()).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(7), Some(7)]
    );
}

fn scalar_failing_expression() -> Box<dyn Expression> {
    Box::new(build_numeric_binary_expression(
        "scalar_failing",
        ArithmeticOperator::Divide,
        PhysicalType::Int32,
        PhysicalType::Int32,
        PhysicalType::Int32,
    ))
}

#[test]
fn every_async_path_preserves_the_exact_scalar_evaluation_error() {
    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(0), 1),
    ];
    let expected = Err(ExpressionError::ScalarEvaluation {
        function: "scalar_failing",
        row: 0,
        error: ScalarError::DivisionByZero,
    });

    assert_eq!(scalar_failing_expression().evaluate(&inputs), expected);

    let static_expression = scalar_failing_expression();
    let mut static_future = std::pin::pin!(evaluate_static(static_expression.as_ref(), &inputs));
    assert_eq!(poll_ready(static_future.as_mut()), expected);

    let erased = AsyncExpressionAdapter::new(scalar_failing_expression());
    let mut erased_future = erased.evaluate_async(&inputs);
    assert_eq!(poll_ready(erased_future.as_mut()), expected);

    let mut registry = FunctionRegistry::default();
    registry.register_binary("scalar_failing", |left, right| {
        BoundExpression::new(
            scalar_failing_expression(),
            [left, right],
            DataType::Integer,
        )
    });
    let bound = registry
        .bind_binary("scalar_failing", DataType::Integer, DataType::Integer)
        .unwrap();
    let mut bound_future = bound.evaluate_async(&inputs);
    assert_eq!(poll_ready(bound_future.as_mut()), expected);
}

#[test]
fn async_boundary_preserves_arity_type_and_length_errors() {
    let expression = AsyncExpressionAdapter::new(build_builtin_expression("i32_add").unwrap());
    let empty = [];
    let mut future = expression.evaluate_async(&empty);
    assert_eq!(
        poll_ready(future.as_mut()),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 0,
        })
    );

    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong"), Some("type")]).into();
    let wrong_type = [
        ColumnViewImpl::array(&strings),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
    ];
    let mut future = expression.evaluate_async(&wrong_type);
    assert_eq!(
        poll_ready(future.as_mut()),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }))
    );

    let integers: ArrayImpl = I32Array::from_values(vec![1, 2]).into();
    let wrong_length = [
        ColumnViewImpl::array(&integers),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
    ];
    let mut future = expression.evaluate_async(&wrong_length);
    assert_eq!(
        poll_ready(future.as_mut()),
        Err(ExpressionError::InputLengthMismatch {
            expected: 2,
            actual: 1,
            input_index: 1,
        })
    );
}

#[test]
fn future_lifetime_covers_the_expression_views_and_arrays() {
    let values: ArrayImpl = I32Array::from_values(vec![5]).into();
    let inputs = [
        ColumnViewImpl::array(&values),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(6), 1),
    ];
    let expression = AsyncExpressionAdapter::new(build_builtin_expression("i32_add").unwrap());

    let mut future = borrowed_erased_future(&expression, &inputs);
    assert_eq!(
        poll_ready(future.as_mut()).unwrap().get(0),
        Some(ScalarRefImpl::Int32(11))
    );
}
