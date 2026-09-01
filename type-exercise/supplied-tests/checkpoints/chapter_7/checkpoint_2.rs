use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    Array, ArrayImpl, BinaryScalarFunction, ColumnViewImpl, I32Add, I32Array, PhysicalType,
    PrimitiveBinaryExpression, ScalarRefImpl,
};

fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

#[test]
fn checkpoint_1_evaluates_dense_and_typed_null_inputs() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let left: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), None]).into();
    let cases = [
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
        (
            [
                ColumnViewImpl::array(&left),
                ColumnViewImpl::null(PhysicalType::Int32, 3),
            ],
            vec![None, None, None],
        ),
    ];

    for (inputs, expected) in cases {
        let output = expression.evaluate(&inputs).unwrap();
        assert_eq!(i32_values(&output), expected);
    }
}

#[test]
fn checkpoint_1_evaluates_indexed_inputs_through_public_behavior() {
    let dictionary: ArrayImpl = I32Array::from_slice(&[Some(4), None, Some(8)]).into();
    let keys = [2, 1, 0];
    let one: ArrayImpl = I32Array::from_values(vec![1, 1, 1]).into();
    let output = PrimitiveBinaryExpression::new("i32_add", I32Add)
        .evaluate(&[
            ColumnViewImpl::indexed(&keys, &dictionary).unwrap(),
            ColumnViewImpl::array(&one),
        ])
        .unwrap();

    assert_eq!(i32_values(&output), vec![Some(9), None, Some(5)]);
}

#[test]
fn checkpoint_2_combines_validity_across_multiple_storage_words() {
    let left_values = (0..137)
        .map(|row| (row % 3 != 0).then_some(row))
        .collect::<Vec<_>>();
    let right_values = (0..137)
        .map(|row| (row % 5 != 0).then_some(1000 + row))
        .collect::<Vec<_>>();
    let left: ArrayImpl = I32Array::from_slice(&left_values).into();
    let right: ArrayImpl = I32Array::from_slice(&right_values).into();
    let output = PrimitiveBinaryExpression::new("i32_add", I32Add)
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    let expected = left_values
        .iter()
        .zip(&right_values)
        .map(|(left, right)| {
            left.zip(*right)
                .map(|(left, right)| left.wrapping_add(right))
        })
        .collect::<Vec<_>>();

    assert_eq!(i32_values(&output), expected);
    assert!(expected.len() > usize::BITS as usize * 2);
}

struct CountingAdd {
    calls: Arc<AtomicUsize>,
}

impl BinaryScalarFunction for CountingAdd {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate(&self, left: i32, right: i32) -> i32 {
        self.calls.fetch_add(1, Ordering::SeqCst);
        left.wrapping_add(right)
    }
}

#[test]
fn checkpoint_2_calls_constant_kernel_once_or_zero_times() {
    let calls = Arc::new(AtomicUsize::new(0));
    let expression = PrimitiveBinaryExpression::new(
        "counting_add",
        CountingAdd {
            calls: Arc::clone(&calls),
        },
    );
    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 65),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 65),
        ])
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(i32_values(&output), vec![Some(7); 65]);

    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 0),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 0),
        ])
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(i32_values(&output).is_empty());
}
