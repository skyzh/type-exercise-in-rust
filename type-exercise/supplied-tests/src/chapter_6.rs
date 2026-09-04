use crate::{
    Array, ArrayImpl, BoolArray, ColumnViewImpl, I32Array, ScalarRefImpl,
    auto_vectorize_primitive_i32, evaluate_nullable_binary, try_evaluate_binary,
};

#[test]
fn keeps_the_raw_int32_lane_bounded_to_non_indexed_int32() {
    let left: ArrayImpl = I32Array::from_slice(&[Some(i32::MAX), None, Some(4)]).into();
    let output = auto_vectorize_primitive_i32(
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
        i32::wrapping_add,
    )
    .unwrap();
    assert_eq!(
        I32Array::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(i32::MIN), None, Some(5)]
    );

    let indices = [2, 0, 1];
    let output = auto_vectorize_primitive_i32(
        ColumnViewImpl::indexed(&indices, &left).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
        i32::wrapping_mul,
    )
    .unwrap();
    assert_eq!(
        I32Array::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(8), Some(-2), None]
    );
}

#[test]
fn reports_the_first_fallible_row_without_publishing_partial_output() {
    let left: ArrayImpl = I32Array::from_slice(&[Some(8), Some(9), Some(10)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(2), Some(0), Some(5)]).into();
    let error = try_evaluate_binary::<i32, i32, i32, _, _>(
        ColumnViewImpl::array(&left),
        ColumnViewImpl::array(&right),
        "divide",
        |left, right| left.checked_div(right).ok_or("division by zero"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("row 1"));
    assert!(error.to_string().contains("division by zero"));
}

#[test]
fn supports_nullable_aware_boolean_semantics_without_a_policy_enum() {
    let left: ArrayImpl = BoolArray::from_slice(&[Some(false), Some(true), None]).into();
    let right: ArrayImpl = BoolArray::from_slice(&[None, None, Some(false)]).into();
    let output = evaluate_nullable_binary::<bool, bool, bool, _>(
        ColumnViewImpl::array(&left),
        ColumnViewImpl::array(&right),
        |left, right| {
            Ok(match (left, right) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            })
        },
    )
    .unwrap();
    assert_eq!(
        BoolArray::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), None, Some(false)]
    );
}
