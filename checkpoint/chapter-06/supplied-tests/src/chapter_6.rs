use std::cell::Cell;

use crate::{
    Array, ArrayImpl, BoolArray, ColumnViewImpl, I32Array, ScalarRefImpl, StringArray,
    auto_vectorize_binary, auto_vectorize_primitive_i32, evaluate_nullable_binary,
    try_evaluate_binary,
};

fn i32_values(array: ArrayImpl) -> Vec<Option<i32>> {
    I32Array::try_from(array).unwrap().iter().collect()
}

fn bool_values(array: ArrayImpl) -> Vec<Option<bool>> {
    BoolArray::try_from(array).unwrap().iter().collect()
}

#[test]
fn primitive_i32_preserves_nullable_results_and_indexed_equivalence() {
    let left: ArrayImpl = I32Array::from_slice(&[Some(2), None, Some(7)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(10), Some(20), None]).into();

    assert_eq!(
        i32_values(
            auto_vectorize_primitive_i32(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::array(&right),
                i32::wrapping_add,
            )
            .unwrap(),
        ),
        vec![Some(12), None, None]
    );
    assert_eq!(
        i32_values(
            auto_vectorize_primitive_i32(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 3),
                i32::wrapping_mul,
            )
            .unwrap(),
        ),
        vec![Some(6), None, Some(21)]
    );
    assert_eq!(
        i32_values(
            auto_vectorize_primitive_i32(
                ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 3),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 3),
                i32::wrapping_add,
            )
            .unwrap(),
        ),
        vec![Some(7), Some(7), Some(7)]
    );

    let indices = [2, 0, 1];
    let specialized = auto_vectorize_primitive_i32(
        ColumnViewImpl::indexed(&indices, &left).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
        i32::wrapping_add,
    )
    .unwrap();
    let generic = auto_vectorize_binary::<i32, i32, i32, _>(
        ColumnViewImpl::indexed(&indices, &left).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
        i32::wrapping_add,
    )
    .unwrap();
    assert_eq!(i32_values(specialized), i32_values(generic));

    let calls = Cell::new(0);
    let short: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    assert!(
        auto_vectorize_primitive_i32(
            ColumnViewImpl::array(&left),
            ColumnViewImpl::array(&short),
            |left, right| {
                calls.set(calls.get() + 1);
                left + right
            },
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn strict_fallible_binary_validates_skips_nulls_and_stops_at_first_error() {
    let left: ArrayImpl = I32Array::from_slice(&[Some(8), None, Some(9), Some(4)]).into();
    let safe_right: ArrayImpl = I32Array::from_slice(&[Some(2), Some(0), Some(3), Some(2)]).into();
    assert_eq!(
        i32_values(
            try_evaluate_binary::<i32, i32, i32, _, _>(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::array(&safe_right),
                "checked_divide",
                |left, right| {
                    if right == 0 {
                        Err("division by zero")
                    } else {
                        Ok(left / right)
                    }
                },
            )
            .unwrap(),
        ),
        vec![Some(4), None, Some(3), Some(2)]
    );

    let right: ArrayImpl = I32Array::from_slice(&[Some(2), Some(0), Some(0), Some(2)]).into();
    let calls = Cell::new(0);
    let error = try_evaluate_binary::<i32, i32, i32, _, _>(
        ColumnViewImpl::array(&left),
        ColumnViewImpl::array(&right),
        "checked_divide",
        |left, right| {
            calls.set(calls.get() + 1);
            if right == 0 {
                Err("division by zero")
            } else {
                Ok(left / right)
            }
        },
    )
    .unwrap_err()
    .to_string();
    assert_eq!(calls.get(), 2);
    assert!(error.contains("checked_divide"));
    assert!(error.contains('2'));

    let invalid_calls = Cell::new(0);
    let short: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    assert!(
        try_evaluate_binary::<i32, i32, i32, _, &str>(
            ColumnViewImpl::array(&left),
            ColumnViewImpl::array(&short),
            "checked_divide",
            |left, right| {
                invalid_calls.set(invalid_calls.get() + 1);
                Ok(left / right)
            },
        )
        .is_err()
    );
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong"); 4]).into();
    assert!(
        try_evaluate_binary::<i32, i32, i32, _, &str>(
            ColumnViewImpl::array(&strings),
            ColumnViewImpl::array(&right),
            "checked_divide",
            |left, right| {
                invalid_calls.set(invalid_calls.get() + 1);
                Ok(left / right)
            },
        )
        .is_err()
    );
    assert_eq!(invalid_calls.get(), 0);
}

fn sql_and(left: Option<bool>, right: Option<bool>) -> anyhow::Result<Option<bool>> {
    Ok(match (left, right) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    })
}

fn sql_or(left: Option<bool>, right: Option<bool>) -> anyhow::Result<Option<bool>> {
    Ok(match (left, right) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    })
}

#[test]
fn nullable_aware_binary_expresses_three_valued_boolean_logic() {
    let left: ArrayImpl =
        BoolArray::from_slice(&[Some(false), Some(false), Some(true), Some(true)]).into();
    let right: ArrayImpl = BoolArray::from_slice(&[Some(true), None, Some(false), None]).into();

    assert_eq!(
        bool_values(
            evaluate_nullable_binary::<bool, bool, bool, _>(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::array(&right),
                sql_and,
            )
            .unwrap(),
        ),
        vec![Some(false), Some(false), Some(false), None]
    );
    assert_eq!(
        bool_values(
            evaluate_nullable_binary::<bool, bool, bool, _>(
                ColumnViewImpl::array(&left),
                ColumnViewImpl::array(&right),
                sql_or,
            )
            .unwrap(),
        ),
        vec![Some(true), None, Some(true), Some(true)]
    );

    let calls = Cell::new(0);
    let indices = [3, 1, 0];
    assert_eq!(
        bool_values(
            evaluate_nullable_binary::<bool, bool, bool, _>(
                ColumnViewImpl::indexed(&indices, &right).unwrap(),
                ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 3),
                |left, right| {
                    calls.set(calls.get() + 1);
                    sql_and(left, right)
                },
            )
            .unwrap(),
        ),
        vec![None, None, Some(true)]
    );
    assert_eq!(calls.get(), 3);

    let invalid_calls = Cell::new(0);
    let short: ArrayImpl = BoolArray::from_slice(&[Some(true)]).into();
    assert!(
        evaluate_nullable_binary::<bool, bool, bool, _>(
            ColumnViewImpl::array(&left),
            ColumnViewImpl::array(&short),
            |left, right| {
                invalid_calls.set(invalid_calls.get() + 1);
                sql_and(left, right)
            },
        )
        .is_err()
    );
    assert_eq!(invalid_calls.get(), 0);
}
