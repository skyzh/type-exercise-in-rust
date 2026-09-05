use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    Array, ArrayImpl, ColumnViewImpl, I16Array, I32Array, I64Array, ScalarRefImpl, StringArray,
    auto_vectorize_binary, auto_vectorize_ternary, auto_vectorize_unary, evaluate_binary,
    evaluate_ternary, evaluate_unary,
};

fn i32_values(array: ArrayImpl) -> Vec<Option<i32>> {
    I32Array::try_from(array).unwrap().iter().collect()
}

fn i64_values(array: ArrayImpl) -> Vec<Option<i64>> {
    I64Array::try_from(array).unwrap().iter().collect()
}

#[test]
fn common_array_and_constant_shapes_preserve_nullable_results() {
    let left: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(5)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(10), Some(20), Some(30)]).into();

    assert_eq!(
        i32_values(
            auto_vectorize_unary::<i32, i32, _>(ColumnViewImpl::array(&left), i32::wrapping_neg)
                .unwrap(),
        ),
        vec![Some(-1), None, Some(-5)]
    );
    assert_eq!(
        i32_values(
            auto_vectorize_unary::<i32, i32, _>(
                ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 3),
                |value| value + 1,
            )
            .unwrap(),
        ),
        vec![Some(8), Some(8), Some(8)]
    );

    for (left_view, right_view, expected) in [
        (
            ColumnViewImpl::array(&left),
            ColumnViewImpl::array(&right),
            vec![Some(11), None, Some(35)],
        ),
        (
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
            vec![Some(3), None, Some(7)],
        ),
        (
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
            ColumnViewImpl::array(&right),
            vec![Some(12), Some(22), Some(32)],
        ),
        (
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 3),
            vec![Some(5), Some(5), Some(5)],
        ),
    ] {
        assert_eq!(
            i32_values(
                auto_vectorize_binary::<i32, i32, i32, _>(
                    left_view,
                    right_view,
                    i32::wrapping_add,
                )
                .unwrap(),
            ),
            expected
        );
    }

    assert_eq!(
        i32_values(
            auto_vectorize_ternary::<i32, i32, i32, i32, _>(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::array(&right),
                ColumnViewImpl::array(&right),
                |first, second, third| first + second + third,
            )
            .unwrap(),
        ),
        vec![Some(21), None, Some(65)]
    );
}

#[test]
fn indexed_and_mixed_shapes_match_the_shared_typed_fallback() {
    let input: ArrayImpl = I32Array::from_slice(&[Some(3), None, Some(8)]).into();
    let indices = [2, 0, 1];
    let auto = auto_vectorize_unary::<i32, i32, _>(
        ColumnViewImpl::indexed(&indices, &input).unwrap(),
        i32::wrapping_neg,
    )
    .unwrap();
    let fallback = evaluate_unary::<i32, i32, _>(
        ColumnViewImpl::indexed(&indices, &input).unwrap(),
        i32::wrapping_neg,
    )
    .unwrap();
    assert_eq!(i32_values(auto), i32_values(fallback));

    let mixed: ArrayImpl = I16Array::from_slice(&[Some(3), None, Some(8)]).into();
    let auto = auto_vectorize_binary::<i16, i32, i32, _>(
        ColumnViewImpl::indexed(&indices, &mixed).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
        |left, right| i32::from(left) + right,
    )
    .unwrap();
    let fallback = evaluate_binary::<i16, i32, i32, _>(
        ColumnViewImpl::indexed(&indices, &mixed).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
        |left, right| i32::from(left) + right,
    )
    .unwrap();
    assert_eq!(i32_values(auto), i32_values(fallback));

    let first: ArrayImpl = I16Array::from_slice(&[Some(1), Some(2), None]).into();
    let third: ArrayImpl = I64Array::from_slice(&[Some(100), None, Some(300)]).into();
    let auto = auto_vectorize_ternary::<i16, i32, i64, i64, _>(
        ColumnViewImpl::array(&first),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
        ColumnViewImpl::array(&third),
        |first, second, third| i64::from(first) + i64::from(second) + third,
    )
    .unwrap();
    let fallback = evaluate_ternary::<i16, i32, i64, i64, _>(
        ColumnViewImpl::array(&first),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
        ColumnViewImpl::array(&third),
        |first, second, third| i64::from(first) + i64::from(second) + third,
    )
    .unwrap();
    assert_eq!(i64_values(auto), i64_values(fallback));
}

#[test]
fn auto_vectorizers_reject_invalid_inputs_before_calling_the_function() {
    let calls = AtomicUsize::new(0);
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong"), Some("type")]).into();
    assert!(
        auto_vectorize_unary::<i32, i32, _>(ColumnViewImpl::array(&strings), |value| {
            calls.fetch_add(1, Ordering::SeqCst);
            value
        })
        .is_err()
    );

    let short: ArrayImpl = I16Array::from_slice(&[Some(1), Some(2)]).into();
    assert!(
        auto_vectorize_binary::<i16, i32, i32, _>(
            ColumnViewImpl::array(&short),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
            |left, right| {
                calls.fetch_add(1, Ordering::SeqCst);
                i32::from(left) + right
            },
        )
        .is_err()
    );
    assert!(
        auto_vectorize_ternary::<i16, i32, i64, i64, _>(
            ColumnViewImpl::array(&short),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
            ColumnViewImpl::array(&strings),
            |first, second, third| {
                calls.fetch_add(1, Ordering::SeqCst);
                i64::from(first) + i64::from(second) + third
            },
        )
        .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
