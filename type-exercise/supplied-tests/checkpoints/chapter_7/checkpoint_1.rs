use crate::{
    Array, ArrayImpl, ColumnViewImpl, I32Add, I32Array, PhysicalType,
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
