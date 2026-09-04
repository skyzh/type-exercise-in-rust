use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use type_exercise_expr::{
    Array, ArrayBuilder, ArrayImpl, AsyncExpression, AsyncExpressionAdapter, BatchExpression,
    BoolArray, ColumnViewImpl, DataType, Expression, F64Array, FunctionRegistry, I16Array,
    I32Array, ListArray, PhysicalType, ScalarRefImpl, StringArray, StringArrayBuilder,
};

fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

fn f64_values(array: &ArrayImpl) -> Vec<Option<f64>> {
    <&F64Array>::try_from(array).unwrap().iter().collect()
}

fn bool_values(array: &ArrayImpl) -> Vec<Option<bool>> {
    <&BoolArray>::try_from(array).unwrap().iter().collect()
}

fn string_values(array: &ArrayImpl) -> Vec<Option<&str>> {
    <&StringArray>::try_from(array).unwrap().iter().collect()
}

#[test]
fn numeric_binary_handles_every_dense_shape_and_indexed_fallback() {
    let add = FunctionRegistry::with_builtins()
        .bind("+", &[DataType::Integer, DataType::Integer])
        .unwrap();
    let left: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), None]).into();

    let dense_cases = [
        (
            [ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)],
            vec![Some(11), None, None],
        ),
        (
            [
                ColumnViewImpl::array(&left),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
            ],
            vec![Some(15), None, Some(35)],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
                ColumnViewImpl::array(&right),
            ],
            vec![Some(6), Some(7), None],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 3),
            ],
            vec![Some(12), Some(12), Some(12)],
        ),
    ];
    for (inputs, expected) in dense_cases {
        assert_eq!(i32_values(&add.evaluate(&inputs).unwrap()), expected);
    }

    let dictionary: ArrayImpl = I32Array::from_slice(&[Some(4), Some(8), None]).into();
    let indices = [1, 2, 0];
    let indexed = ColumnViewImpl::indexed(&indices, &dictionary).unwrap();
    let output = add
        .evaluate(&[
            indexed,
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
        ])
        .unwrap();
    assert_eq!(i32_values(&output), vec![Some(9), None, Some(5)]);
}

#[test]
fn mixed_numeric_instantiation_preserves_promotion_and_nulls() {
    let add = FunctionRegistry::with_builtins()
        .bind("+", &[DataType::SmallInt, DataType::Double])
        .unwrap();
    assert_eq!(add.output_type(), DataType::Double);
    let left: ArrayImpl = I16Array::from_slice(&[Some(2), None, Some(-4)]).into();
    let output = add
        .evaluate(&[
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::Float64(0.5), 3),
        ])
        .unwrap();
    assert_eq!(f64_values(&output), vec![Some(2.5), None, Some(-3.5)]);
}

#[test]
fn non_commutative_operators_preserve_left_and_right_order() {
    let registry = FunctionRegistry::with_builtins();

    let subtract = registry
        .bind("-", &[DataType::SmallInt, DataType::Double])
        .unwrap();
    let left: ArrayImpl = I16Array::from_values(vec![10, -3]).into();
    let right: ArrayImpl = F64Array::from_values(vec![4.5, 8.0]).into();
    assert_eq!(
        f64_values(
            &subtract
                .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right),])
                .unwrap()
        ),
        vec![Some(5.5), Some(-11.0)]
    );

    let numeric_less = registry
        .bind("<", &[DataType::SmallInt, DataType::Double])
        .unwrap();
    assert_eq!(
        bool_values(
            &numeric_less
                .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right),])
                .unwrap()
        ),
        vec![Some(false), Some(true)]
    );

    let string_less = registry
        .bind("<", &[DataType::Varchar, DataType::Varchar])
        .unwrap();
    let string_left: ArrayImpl = StringArray::from_slice(&[Some("ant"), Some("zebra")]).into();
    let string_right: ArrayImpl = StringArray::from_slice(&[Some("bee"), Some("yak")]).into();
    assert_eq!(
        bool_values(
            &string_less
                .evaluate(&[
                    ColumnViewImpl::array(&string_left),
                    ColumnViewImpl::array(&string_right),
                ])
                .unwrap()
        ),
        vec![Some(true), Some(false)]
    );
}

#[test]
fn unary_and_ternary_cover_dense_and_fallback_shapes() {
    let registry = FunctionRegistry::with_builtins();
    let neg = registry.bind("neg", &[DataType::Integer]).unwrap();
    let values: ArrayImpl = I32Array::from_slice(&[Some(4), None, Some(-7)]).into();
    assert_eq!(
        i32_values(&neg.evaluate(&[ColumnViewImpl::array(&values)]).unwrap()),
        vec![Some(-4), None, Some(7)]
    );
    assert_eq!(
        i32_values(
            &neg.evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 3,)])
                .unwrap()
        ),
        vec![Some(-3); 3]
    );
    let indices = [2, 0, 1];
    let indexed = ColumnViewImpl::indexed(&indices, &values).unwrap();
    assert_eq!(
        i32_values(&neg.evaluate(&[indexed]).unwrap()),
        vec![Some(7), Some(-4), None]
    );

    let clamp = registry
        .bind(
            "clamp",
            &[DataType::Integer, DataType::Integer, DataType::Integer],
        )
        .unwrap();
    let lower: ArrayImpl = I32Array::from_values(vec![0, 0, 0]).into();
    let upper: ArrayImpl = I32Array::from_values(vec![5, 5, 5]).into();
    let dense = clamp
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::array(&lower),
            ColumnViewImpl::array(&upper),
        ])
        .unwrap();
    assert_eq!(i32_values(&dense), vec![Some(4), None, Some(0)]);

    let fallback = clamp
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(0), 3),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
        ])
        .unwrap();
    assert_eq!(i32_values(&fallback), vec![Some(4), None, Some(0)]);
}

#[test]
fn semantic_exceptions_keep_row_errors_and_three_valued_logic() {
    let registry = FunctionRegistry::with_builtins();
    let divide = registry
        .bind("/", &[DataType::Integer, DataType::Integer])
        .unwrap();
    let divisors: ArrayImpl = I32Array::from_slice(&[None, Some(0)]).into();
    let error = divide
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(8), 2),
            ColumnViewImpl::array(&divisors),
        ])
        .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("row 1"), "{message}");
    assert!(message.contains("division by zero"), "{message}");

    let boolean_and = registry
        .bind("boolean_and", &[DataType::Boolean, DataType::Boolean])
        .unwrap();
    let left: ArrayImpl = BoolArray::from_slice(&[None, None, Some(true)]).into();
    let right: ArrayImpl = BoolArray::from_slice(&[Some(false), Some(true), None]).into();
    let output = boolean_and
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(bool_values(&output), vec![Some(false), None, None]);
}

#[test]
fn writer_and_borrowed_string_functions_share_column_views() {
    let registry = FunctionRegistry::with_builtins();
    let concat = registry
        .bind("concat", &[DataType::Varchar, DataType::Varchar])
        .unwrap();
    let values: ArrayImpl = StringArray::from_slice(&[Some("ab"), None, Some("cd")]).into();
    let output = concat
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::String("!"), 3),
        ])
        .unwrap();
    assert_eq!(string_values(&output), vec![Some("ab!"), None, Some("cd!")]);

    let contains = registry
        .bind("contains", &[DataType::Varchar, DataType::Varchar])
        .unwrap();
    let output = contains
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::String("b"), 3),
        ])
        .unwrap();
    assert_eq!(bool_values(&output), vec![Some(true), None, Some(false)]);
}

#[test]
fn batch_boundary_reports_public_arity_type_and_length_errors() {
    let add = FunctionRegistry::with_builtins()
        .bind("+", &[DataType::Integer, DataType::Integer])
        .unwrap();
    let integers: ArrayImpl = I32Array::from_values(vec![1, 2]).into();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong"), Some("type")]).into();

    assert_eq!(
        add.evaluate(&[ColumnViewImpl::array(&integers)])
            .unwrap_err()
            .to_string(),
        "input arity mismatch: expected 2, got 1"
    );
    assert_eq!(
        add.evaluate(&[
            ColumnViewImpl::array(&strings),
            ColumnViewImpl::array(&integers),
        ])
        .unwrap_err()
        .to_string(),
        "input 0 type mismatch: expected Int32, got String"
    );
    assert_eq!(
        add.evaluate(&[
            ColumnViewImpl::array(&integers),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ])
        .unwrap_err()
        .to_string(),
        "input 1 length mismatch: expected 2, got 1"
    );

    fn short_kernel(_inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        Ok(I32Array::from_values(vec![99]).into())
    }
    let short = BatchExpression::new(
        "short",
        [PhysicalType::Int32],
        PhysicalType::Int32,
        short_kernel,
    );
    let three_rows: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    assert_eq!(
        short
            .evaluate(&[ColumnViewImpl::array(&three_rows)])
            .unwrap_err()
            .to_string(),
        "output length mismatch: expected 3, got 1"
    );
}

struct CountedExpression {
    calls: Arc<AtomicUsize>,
}

impl Expression for CountedExpression {
    fn name(&self) -> &'static str {
        "counted"
    }

    fn input_types(&self) -> &[PhysicalType] {
        &[PhysicalType::Int32]
    }

    fn output_type(&self) -> PhysicalType {
        PhysicalType::Int32
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(I32Array::from_values(vec![7; inputs[0].len()]).into())
    }
}

#[test]
fn async_adapter_evaluates_its_child_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let expression = AsyncExpressionAdapter::new(Box::new(CountedExpression {
        calls: Arc::clone(&calls),
    }));
    let values: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let inputs = [ColumnViewImpl::array(&values)];
    let mut future = expression.evaluate_async(&inputs);
    let mut context = Context::from_waker(Waker::noop());
    let Poll::Ready(output) = Future::poll(future.as_mut(), &mut context) else {
        panic!("an adapter around synchronous evaluation must be ready after one poll");
    };

    assert_eq!(i32_values(&output.unwrap()), vec![Some(7); 3]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_string_write_rolls_back_pending_bytes_and_row_metadata() {
    let mut builder = StringArrayBuilder::with_capacity(1);
    let error = builder
        .try_push_with(|writer| {
            writer.push_str("discarded");
            Err::<(), _>("write failed")
        })
        .unwrap_err();
    assert_eq!(error, "write failed");
    builder
        .try_push_with(|writer| {
            writer.push_str("kept");
            Ok::<(), &str>(())
        })
        .unwrap();

    let output = builder.finish();
    assert_eq!(output.data(), b"kept");
    assert_eq!(output.offsets(), &[0, 4]);
    assert_eq!(output.len(), 1);
    assert_eq!(output.get(0), Some("kept"));
}

#[test]
fn one_level_list_exposes_valid_rows_and_nullable_elements() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
    let lists = ListArray::try_from_raw_parts(
        PhysicalType::Int32,
        values,
        vec![0, 2, 2, 3],
        vec![true, false, true],
    )
    .unwrap();
    let lists: ArrayImpl = lists.into();
    let view = ColumnViewImpl::array(&lists)
        .try_as_list(PhysicalType::Int32)
        .unwrap();

    let first = view.get(0).unwrap().unwrap();
    assert_eq!(first.element_type(), PhysicalType::Int32);
    assert_eq!(first.len(), 2);
    assert_eq!(first.get(0).unwrap(), Some(ScalarRefImpl::Int32(1)));
    assert_eq!(first.get(1).unwrap(), None);
    assert_eq!(view.get(1).unwrap(), None);
    let third = view.get(2).unwrap().unwrap();
    assert_eq!(third.get(0).unwrap(), Some(ScalarRefImpl::Int32(3)));
}
