use crate::{
    Array, ArrayImpl, ColumnViewImpl, I32Add, I32Array, PrimitiveBinaryExpression, PrimitiveLoop,
    PhysicalType, ScalarRefImpl,
};

fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

#[test]
fn checkpoint_1_preserves_array_constant_and_null_rows() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(4), None, Some(8)]).into();
    let array = ColumnViewImpl::array(&values);
    assert_eq!(array.len(), 3);
    assert_eq!(array.physical_type(), PhysicalType::Int32);
    assert_eq!(array.get(0), Some(ScalarRefImpl::Int32(4)));
    assert_eq!(array.get(1), None);

    let constant = ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 3);
    assert_eq!(constant.len(), 3);
    assert_eq!(constant.get(2), Some(ScalarRefImpl::Int32(7)));

    let null = ColumnViewImpl::null(PhysicalType::Int32, 3);
    assert_eq!(null.len(), 3);
    assert_eq!(null.get(2), None);
}

#[test]
fn checkpoint_1_preserves_indexed_order_nulls_and_bounds() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(4), None, Some(8)]).into();
    let keys = [2, 1, 0];
    let indexed = ColumnViewImpl::indexed(&keys, &values).unwrap();
    assert_eq!(indexed.len(), 3);
    assert_eq!(indexed.physical_type(), PhysicalType::Int32);
    assert_eq!(indexed.get(0), Some(ScalarRefImpl::Int32(8)));
    assert_eq!(indexed.get(1), None);
    assert_eq!(indexed.get(2), Some(ScalarRefImpl::Int32(4)));

    assert_eq!(
        ColumnViewImpl::indexed(&[3], &values)
            .unwrap_err()
            .to_string(),
        "index 3 at row 0 is out of bounds for a values array of length 3"
    );
}

#[test]
fn checkpoint_2_selects_all_four_raw_shapes() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let left: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), None]).into();
    let cases = [
        (
            [ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)],
            PrimitiveLoop::ArrayArray,
            vec![Some(11), None, None],
        ),
        (
            [
                ColumnViewImpl::array(&left),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
            ],
            PrimitiveLoop::ArrayConstant,
            vec![Some(15), None, Some(35)],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
                ColumnViewImpl::array(&right),
            ],
            PrimitiveLoop::ConstantArray,
            vec![Some(6), Some(7), None],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 3),
            ],
            PrimitiveLoop::ConstantConstant,
            vec![Some(12), Some(12), Some(12)],
        ),
        (
            [
                ColumnViewImpl::array(&left),
                ColumnViewImpl::null(PhysicalType::Int32, 3),
            ],
            PrimitiveLoop::ArrayConstant,
            vec![None, None, None],
        ),
    ];

    for (inputs, expected_loop, expected_values) in cases {
        let (output, selected) = expression.evaluate_with_loop(&inputs).unwrap();
        assert_eq!(selected, expected_loop);
        assert_eq!(i32_values(&output), expected_values);
    }
}

#[test]
fn checkpoint_2_combines_validity_by_storage_word_and_falls_back_for_indexed() {
    let left_values = (0..137)
        .map(|row| (row % 3 != 0).then_some(row))
        .collect::<Vec<_>>();
    let right_values = (0..137)
        .map(|row| (row % 5 != 0).then_some(1000 + row))
        .collect::<Vec<_>>();
    let left: ArrayImpl = I32Array::from_slice(&left_values).into();
    let right: ArrayImpl = I32Array::from_slice(&right_values).into();
    let (output, selected) = PrimitiveBinaryExpression::new("i32_add", I32Add)
        .evaluate_with_loop(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::ArrayArray);
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

    let dictionary: ArrayImpl = I32Array::from_slice(&[Some(4), None, Some(8)]).into();
    let keys = [2, 1, 0];
    let one: ArrayImpl = I32Array::from_values(vec![1, 1, 1]).into();
    let (output, selected) = PrimitiveBinaryExpression::new("i32_add", I32Add)
        .evaluate_with_loop(&[
            ColumnViewImpl::indexed(&keys, &dictionary).unwrap(),
            ColumnViewImpl::array(&one),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::Indexed);
    assert_eq!(i32_values(&output), vec![Some(9), None, Some(5)]);
}
