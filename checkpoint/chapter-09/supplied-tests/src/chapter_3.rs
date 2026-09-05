use std::cell::Cell;

use crate::{
    Array, ArrayImpl, ColumnViewImpl, F64Array, I16Array, I32Array, PhysicalType, ScalarRefImpl,
    add_i16_i32, clamp_i32, evaluate_binary, evaluate_unary, negate_i32,
    validate_expression_inputs,
};

fn i32_values(array: ArrayImpl) -> Vec<Option<i32>> {
    I32Array::try_from(array).unwrap().iter().collect()
}

#[test]
fn numeric_facade_instantiates_shared_unary_binary_and_ternary_evaluation() {
    let input: ArrayImpl = I32Array::from_slice(&[Some(2), None, Some(-3)]).into();
    let negated = negate_i32(ColumnViewImpl::array(&input)).unwrap();
    assert_eq!(i32_values(negated), vec![Some(-2), None, Some(3)]);

    let left: ArrayImpl = I16Array::from_slice(&[Some(1), None, Some(4)]).into();
    let added = add_i16_i32(
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
    )
    .unwrap();
    assert_eq!(i32_values(added), vec![Some(11), None, Some(14)]);

    let clamped = clamp_i32(
        ColumnViewImpl::array(&input),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(0), 3),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
    )
    .unwrap();
    assert_eq!(i32_values(clamped), vec![Some(2), None, Some(0)]);
}

#[test]
fn indexed_inputs_use_the_same_fallback_and_skip_null_callbacks() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(3), None, Some(8)]).into();
    let indexed = ColumnViewImpl::indexed(&[2, 0, 1], &values).unwrap();
    let calls = Cell::new(0);
    let doubled = evaluate_unary::<i32, i32, _>(indexed, |value| {
        calls.set(calls.get() + 1);
        value * 2
    })
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(i32_values(doubled), vec![Some(16), Some(6), None]);

    let floats: ArrayImpl = F64Array::from_slice(&[Some(1.5), None, Some(2.0)]).into();
    let indexed = ColumnViewImpl::indexed(&[2, 0, 1], &floats).unwrap();
    let summed = evaluate_binary::<f64, f64, f64, _>(
        indexed,
        ColumnViewImpl::constant(ScalarRefImpl::Float64(0.5), 3),
        |left, right| left + right,
    )
    .unwrap();
    assert_eq!(
        F64Array::try_from(summed)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(2.5), Some(2.0), None]
    );
}

#[test]
fn rejects_arity_type_and_length_mismatches_before_evaluation() {
    assert!(validate_expression_inputs(&[], &[PhysicalType::Int32]).is_err());

    let integers: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2)]).into();
    let short = ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1);
    assert!(
        evaluate_binary::<i32, i32, i32, _>(
            ColumnViewImpl::array(&integers),
            short,
            |left, right| left + right,
        )
        .is_err()
    );

    let wrong = ColumnViewImpl::null(PhysicalType::Float64, 2);
    assert!(evaluate_unary::<i32, i32, _>(wrong, |value| value).is_err());
}
