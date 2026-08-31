use crate::{
    Array, ArrayImpl, ColumnViewImpl, I32Add, I32Array, Nullability, PhysicalType,
    PrimitiveBinaryExpression, PrimitiveLoop, ScalarRefImpl,
};

fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

#[test]
fn checkpoint_1_classifies_physical_nullability() {
    let dense: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    assert_eq!(
        ColumnViewImpl::array(&dense).nullability(),
        Nullability::Nullable
    );
    assert_eq!(
        ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 2).nullability(),
        Nullability::NonNull
    );
    assert_eq!(
        ColumnViewImpl::null(PhysicalType::Int32, 2).nullability(),
        Nullability::Nullable
    );
    let indices = [0];
    assert_eq!(
        ColumnViewImpl::indexed(&indices, &dense)
            .unwrap()
            .nullability(),
        Nullability::Nullable
    );
}

#[test]
fn checkpoint_1_recovers_only_all_valid_fixed_width_arrays() {
    let nullable: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
    assert!(ColumnViewImpl::try_non_null_array(&nullable).is_none());

    let dense: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    assert_eq!(
        ColumnViewImpl::try_non_null_array(&dense)
            .unwrap()
            .nullability(),
        Nullability::NonNull
    );

    let built_dense: ArrayImpl = I32Array::from_slice(&[Some(4), Some(5)]).into();
    assert!(ColumnViewImpl::try_non_null_array(&built_dense).is_some());
    let empty: ArrayImpl = I32Array::from_values(Vec::new()).into();
    assert!(ColumnViewImpl::try_non_null_array(&empty).is_some());
}

#[test]
fn checkpoint_2_selects_the_dense_array_array_loop() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let left: ArrayImpl = I32Array::from_values(vec![i32::MAX, 20, 30]).into();
    let right: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::try_non_null_array(&left).unwrap(),
            ColumnViewImpl::try_non_null_array(&right).unwrap(),
        ])
        .unwrap();

    assert_eq!(selected, PrimitiveLoop::ArrayArray);
    assert_eq!(
        i32_values(&output),
        vec![Some(i32::MIN), Some(22), Some(33)]
    );
}

#[test]
fn checkpoint_2_selects_all_three_dense_constant_loops() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let values: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let cases = [
        (
            [
                ColumnViewImpl::try_non_null_array(&values).unwrap(),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
            ],
            PrimitiveLoop::ArrayConstant,
            vec![Some(11), Some(12), Some(13)],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
                ColumnViewImpl::try_non_null_array(&values).unwrap(),
            ],
            PrimitiveLoop::ConstantArray,
            vec![Some(11), Some(12), Some(13)],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
            ],
            PrimitiveLoop::ConstantConstant,
            vec![Some(12), Some(12), Some(12)],
        ),
    ];

    for (inputs, expected_loop, expected_values) in cases {
        let (output, selected) = expression.evaluate_with_loop(&inputs).unwrap();
        assert_eq!(selected, expected_loop);
        assert_eq!(i32_values(&output), expected_values);
    }
}
