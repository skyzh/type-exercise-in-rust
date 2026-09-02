use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::checkpoint_1::i32_values;
use crate::*;

#[test]
fn combines_nullable_validity_by_storage_word() {
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
fn constant_constant_invokes_the_scalar_once_or_not_at_all() {
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
