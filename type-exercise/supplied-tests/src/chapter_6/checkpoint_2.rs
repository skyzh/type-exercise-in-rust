use std::cell::Cell;

use crate::*;

#[test]
fn strict_vectorization_skips_the_scalar_function_for_null_rows() {
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
fn direct_mixed_batch_kernel_is_strict_and_reports_the_failing_row() {
    let expression = build_numeric_clamp_expression(
        "mixed_clamp",
        [
            PhysicalType::Int16,
            PhysicalType::Int32,
            PhysicalType::Int64,
        ],
        PhysicalType::Int64,
    );
    let values: ArrayImpl = I16Array::from_slice(&[Some(5), None, Some(25)]).into();
    let uppers: ArrayImpl = I64Array::from_values(vec![20, 0, 20]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
            ColumnViewImpl::array(&uppers),
        ])
        .unwrap();
    assert_eq!(
        <&I64Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(10), None, Some(20)]
    );

    let invalid_uppers: ArrayImpl = I64Array::from_values(vec![20, 0]).into();
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int16(5), 2),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 2),
                ColumnViewImpl::array(&invalid_uppers),
            ])
            .unwrap_err()
            .to_string(),
        "function `mixed_clamp` failed at row 1: invalid clamp bounds"
    );
}
