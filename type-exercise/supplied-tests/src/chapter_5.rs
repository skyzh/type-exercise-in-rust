use crate::{
    Array, ArrayImpl, ColumnViewImpl, I32Array, ScalarRefImpl, auto_vectorize_binary,
    auto_vectorize_ternary, auto_vectorize_unary,
};

fn values(array: ArrayImpl) -> Vec<Option<i32>> {
    I32Array::try_from(array).unwrap().iter().collect()
}

#[test]
fn specializes_common_unary_binary_and_ternary_shapes() {
    let left: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(5)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(10), Some(20), Some(30)]).into();

    assert_eq!(
        values(
            auto_vectorize_unary::<i32, i32, _>(ColumnViewImpl::array(&left), |value| -value)
                .unwrap()
        ),
        vec![Some(-1), None, Some(-5)]
    );
    assert_eq!(
        values(
            auto_vectorize_binary::<i32, i32, i32, _>(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
                |left, right| left + right,
            )
            .unwrap()
        ),
        vec![Some(3), None, Some(7)]
    );
    assert_eq!(
        values(
            auto_vectorize_ternary::<i32, i32, i32, i32, _>(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::array(&right),
                ColumnViewImpl::array(&right),
                |first, second, third| first + second + third,
            )
            .unwrap()
        ),
        vec![Some(21), None, Some(65)]
    );
}

#[test]
fn indexed_inputs_keep_the_shared_typed_fallback() {
    let input: ArrayImpl = I32Array::from_slice(&[Some(3), None, Some(8)]).into();
    let indices = [2, 0, 1];
    let output = auto_vectorize_binary::<i32, i32, i32, _>(
        ColumnViewImpl::indexed(&indices, &input).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
        |left, right| left + right,
    )
    .unwrap();
    assert_eq!(values(output), vec![Some(9), Some(4), None]);
}
