use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use crate::*;

fn run_ready<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the batch adapter must not require an external runtime"),
    }
}

#[test]
fn one_level_list_scalars_preserve_nullable_children_and_checked_slices() {
    let scalar =
        ListScalar::try_new(I32Array::from_slice(&[Some(10), None, Some(30)]).into()).unwrap();
    assert_eq!(scalar.element_type(), PhysicalType::Int32);
    assert_eq!(scalar.len(), 3);
    assert_eq!(scalar.get(0).unwrap(), Some(ScalarRefImpl::Int32(10)));
    assert_eq!(scalar.get(1).unwrap(), None);

    let borrowed = scalar.as_list_ref().slice(1, 3).unwrap();
    assert_eq!(borrowed.len(), 2);
    assert_eq!(borrowed.get(0).unwrap(), None);
    assert_eq!(borrowed.get(1).unwrap(), Some(ScalarRefImpl::Int32(30)));
    assert_eq!(borrowed.to_owned_scalar().unwrap().len(), 2);
    assert!(scalar.slice(2, 4).is_err());
    assert!(scalar.get(3).is_err());
}

#[test]
fn list_arrays_distinguish_null_empty_and_nullable_child_rows() {
    let values =
        ListScalar::try_new(I32Array::from_slice(&[Some(1), None, Some(3)]).into()).unwrap();
    let empty = ListScalar::try_new(I32Array::from_slice(&[]).into()).unwrap();
    let array = ListArray::try_from_rows(
        PhysicalType::Int32,
        [Some(values.as_list_ref()), None, Some(empty.as_list_ref())],
    )
    .unwrap();

    assert_eq!(array.element_type(), PhysicalType::Int32);
    assert_eq!(array.offsets(), &[0, 3, 3, 3]);
    assert_eq!(array.validity(), &[true, false, true]);
    assert_eq!(array.get(0).unwrap().unwrap().get(1).unwrap(), None);
    assert_eq!(array.get(1).unwrap(), None);
    assert!(array.get(2).unwrap().unwrap().is_empty());

    let sliced = array.slice(1, 3).unwrap();
    assert_eq!(sliced.offsets(), &[0, 0, 0]);
    assert_eq!(sliced.validity(), &[false, true]);

    let all_null =
        ListArray::try_from_rows(PhysicalType::String, [None::<ListScalarRef<'_>>, None]).unwrap();
    assert_eq!(
        ArrayImpl::from(all_null).physical_type(),
        PhysicalType::List(Box::new(PhysicalType::String))
    );
}

#[test]
fn list_construction_checks_child_descriptors_offsets_and_nesting() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2)]).into();
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::String,
            values.clone(),
            vec![0, 2],
            vec![true],
        )
        .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(PhysicalType::Int32, values.clone(), vec![1, 2], vec![true],)
            .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::Int32,
            values.clone(),
            vec![0, 2],
            vec![false],
        )
        .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(PhysicalType::Int32, values, vec![0, 1], vec![true],)
            .is_err()
    );
    assert!(
        ListArray::try_from_rows(
            PhysicalType::List(Box::new(PhysicalType::Int32)),
            [None::<ListScalarRef<'_>>],
        )
        .is_err()
    );

    let expected = DecimalType::try_new(8, 2).unwrap();
    let actual = DecimalType::try_new(8, 3).unwrap();
    let child =
        DecimalArray::try_from_slice(actual, &[Some(Decimal::try_new(100, actual).unwrap())])
            .unwrap();
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::Decimal(expected),
            child.into(),
            vec![0, 1],
            vec![true],
        )
        .is_err()
    );
}

#[test]
fn list_column_views_preserve_array_constant_and_indexed_rows() {
    let first = ListScalar::try_new(I32Array::from_slice(&[Some(1), None]).into()).unwrap();
    let second = ListScalar::try_new(I32Array::from_slice(&[Some(3)]).into()).unwrap();
    let array = ListArray::try_from_rows(
        PhysicalType::Int32,
        [Some(first.as_list_ref()), None, Some(second.as_list_ref())],
    )
    .unwrap();
    let erased: ArrayImpl = array.into();

    let array_view = ColumnViewImpl::array(&erased)
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(array_view.get(0).unwrap().unwrap().len(), 2);
    assert_eq!(array_view.get(1).unwrap(), None);

    let constant = ColumnViewImpl::constant(first.as_list_ref().into(), 2)
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(
        constant.get(1).unwrap().unwrap().get(0).unwrap(),
        Some(ScalarRefImpl::Int32(1))
    );

    let indices = [2, 0, 1];
    let indexed = ColumnViewImpl::indexed(&indices, &erased)
        .unwrap()
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(indexed.get(0).unwrap().unwrap().len(), 1);
    assert_eq!(indexed.get(2).unwrap(), None);
    assert!(indexed.get(3).is_err());
    assert!(
        ColumnViewImpl::array(&erased)
            .try_as_list(PhysicalType::String)
            .is_err()
    );
}

struct CountingExpression {
    calls: Arc<AtomicUsize>,
}

impl Expression for CountingExpression {
    fn name(&self) -> &'static str {
        "counting"
    }

    fn input_types(&self) -> &[PhysicalType] {
        &[]
    }

    fn output_type(&self) -> PhysicalType {
        PhysicalType::Int32
    }

    fn evaluate(&self, _inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(I32Array::from_slice(&[]).into())
    }
}

#[test]
fn async_boundaries_defer_and_invoke_one_complete_batch_once() {
    fn assert_send<T: Send>(_value: &T) {}

    let calls = Arc::new(AtomicUsize::new(0));
    let expression = CountingExpression {
        calls: calls.clone(),
    };
    let inputs = [];
    let future = evaluate_static(&expression, &inputs);
    assert_send(&future);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(run_ready(future).unwrap().len(), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let adapter = AsyncExpressionAdapter::new(Box::new(CountingExpression {
        calls: calls.clone(),
    }));
    let erased: &dyn AsyncExpression = &adapter;
    let future: BatchFuture<'_> = erased.evaluate_async(&inputs);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(run_ready(future).unwrap().len(), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn bound_batches_match_synchronous_static_and_erased_async_results() {
    let left: ArrayImpl = I16Array::from_slice(&[Some(2), None, Some(-4)]).into();
    let inputs = [
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
    ];

    let bound = bind_logical_call(LogicalCall::new(
        "+",
        [DataType::SmallInt, DataType::Integer],
    ))
    .unwrap();
    let synchronous = bound.evaluate(&inputs).unwrap();
    let compiler_known = run_ready(evaluate_static(bound.physical_expression(), &inputs)).unwrap();
    assert_eq!(compiler_known, synchronous);

    let adapter = AsyncExpressionAdapter::new(bound.into_physical_expression());
    let erased: &dyn AsyncExpression = &adapter;
    let asynchronous = run_ready(erased.evaluate_async(&inputs)).unwrap();
    assert_eq!(asynchronous, synchronous);

    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    let invalid = [
        ColumnViewImpl::array(&strings),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
    ];
    let expression = bind_logical_call(LogicalCall::new(
        "+",
        [DataType::SmallInt, DataType::Integer],
    ))
    .unwrap()
    .into_physical_expression();
    let adapter = AsyncExpressionAdapter::new(expression);
    assert!(run_ready(adapter.evaluate_async(&invalid)).is_err());
}
