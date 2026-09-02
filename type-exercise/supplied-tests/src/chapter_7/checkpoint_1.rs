use crate::*;

pub(super) fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

fn generic_add(inputs: &[ColumnViewImpl<'_>; 2]) -> ArrayImpl {
    auto_vectorize_binary::<i32, i32, i32, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        i32::wrapping_add,
    )
    .unwrap()
}

#[test]
fn evaluates_dense_shapes_including_nullable_arrays_and_typed_nulls() {
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

    for (inputs, expected_values) in cases {
        let generic = generic_add(&inputs);
        let output = expression.evaluate(&inputs).unwrap();
        assert_eq!(i32_values(&output), expected_values);
        assert_eq!(i32_values(&output), i32_values(&generic));
    }
}

#[test]
fn indexed_inputs_preserve_order_and_nulls() {
    let dictionary_values: ArrayImpl = I32Array::from_slice(&[Some(4), Some(8), None]).into();
    let keys = [1, 2, 0];
    let right: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let inputs = [
        ColumnViewImpl::indexed(&keys, &dictionary_values).unwrap(),
        ColumnViewImpl::array(&right),
    ];
    let generic = generic_add(&inputs);
    let output = PrimitiveBinaryExpression::new("i32_add", I32Add)
        .evaluate(&inputs)
        .unwrap();
    assert_eq!(i32_values(&output), vec![Some(9), None, Some(7)]);
    assert_eq!(i32_values(&output), i32_values(&generic));
}
