use std::cell::Cell;

use crate::{
    Array, ArrayImpl, ColumnViewImpl, I16Array, I32Array, I64Array, ScalarRefImpl, evaluate_unary,
    try_evaluate_ternary,
};

#[test]
fn checkpoint_1_strict_vectorization_skips_null_rows() {
    let calls = Cell::new(0);
    let input: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
    let output = evaluate_unary::<i32, i32, _>(ColumnViewImpl::array(&input), |value| {
        calls.set(calls.get() + 1);
        value + 10
    })
    .unwrap();

    assert_eq!(calls.get(), 2);
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(11), None, Some(13)]
    );
}

#[test]
fn checkpoint_2_runs_one_direct_mixed_ternary_evaluator() {
    let values: ArrayImpl = I16Array::from_slice(&[Some(5), None, Some(25)]).into();
    let uppers: ArrayImpl = I64Array::from_values(vec![20, 0, 20]).into();
    let output = try_evaluate_ternary::<i16, i32, i64, i64, _>(
        ColumnViewImpl::array(&values),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
        ColumnViewImpl::array(&uppers),
        "mixed_clamp",
        |value, lower, upper| {
            let value = i64::from(value);
            let lower = i64::from(lower);
            if lower > upper {
                anyhow::bail!("invalid clamp bounds");
            }
            Ok(value.clamp(lower, upper))
        },
    )
    .unwrap();

    assert_eq!(
        <&I64Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(10), None, Some(20)]
    );
}
