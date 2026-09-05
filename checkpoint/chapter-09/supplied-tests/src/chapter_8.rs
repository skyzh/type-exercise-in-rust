use std::cell::Cell;

use crate::*;

fn numeric_output(left: &PhysicalType, right: &PhysicalType) -> Option<PhysicalType> {
    use PhysicalType::*;
    Some(match (left, right) {
        (Int16, Int16) => Int16,
        (Int16, Int32) | (Int32, Int16) | (Int32, Int32) => Int32,
        (Int16, Int64) | (Int64, Int16) | (Int32, Int64) | (Int64, Int32) | (Int64, Int64) => Int64,
        (Int16, Float32) | (Float32, Int16) | (Float32, Float32) => Float32,
        (Int16, Float64)
        | (Float64, Int16)
        | (Int32, Float32)
        | (Float32, Int32)
        | (Int32, Float64)
        | (Float64, Int32)
        | (Float32, Float64)
        | (Float64, Float32)
        | (Float64, Float64) => Float64,
        _ => return None,
    })
}

#[test]
fn fallible_ternary_validates_skips_nulls_and_stops_at_first_error() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(5), None, Some(9), Some(4)]).into();
    let lower: ArrayImpl = I32Array::from_slice(&[Some(0), Some(8), Some(10), Some(6)]).into();
    let upper: ArrayImpl = I32Array::from_slice(&[Some(10), Some(1), Some(11), Some(3)]).into();
    let calls = Cell::new(0);
    assert!(
        try_auto_vectorize_ternary::<i32, i32, i32, i32, _, _>(
            ColumnViewImpl::array(&values),
            ColumnViewImpl::array(&lower),
            ColumnViewImpl::array(&upper),
            "checked_clamp",
            |value, lower, upper| {
                calls.set(calls.get() + 1);
                (lower <= upper)
                    .then(|| value.clamp(lower, upper))
                    .ok_or("invalid bounds")
            },
        )
        .is_err()
    );
    assert_eq!(calls.get(), 3);

    let invalid_calls = Cell::new(0);
    let short: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    assert!(
        try_auto_vectorize_ternary::<i32, i32, i32, i32, _, &str>(
            ColumnViewImpl::array(&values),
            ColumnViewImpl::array(&lower),
            ColumnViewImpl::array(&short),
            "checked_clamp",
            |value, lower, upper| {
                invalid_calls.set(invalid_calls.get() + 1);
                Ok(value.clamp(lower, upper))
            },
        )
        .is_err()
    );
    assert_eq!(invalid_calls.get(), 0);

    let safe_upper: ArrayImpl =
        I32Array::from_slice(&[Some(10), Some(1), Some(11), Some(8)]).into();
    let output = try_auto_vectorize_ternary::<i32, i32, i32, i32, _, &str>(
        ColumnViewImpl::array(&values),
        ColumnViewImpl::array(&lower),
        ColumnViewImpl::array(&safe_upper),
        "checked_clamp",
        |value, lower, upper| Ok(value.clamp(lower, upper)),
    )
    .unwrap();
    assert_eq!(
        I32Array::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(5), None, Some(10), Some(6)]
    );
}

#[test]
fn catalog_discovers_functions_and_selects_every_supported_signature() {
    let expected = [
        PhysicalFunction::Add,
        PhysicalFunction::Subtract,
        PhysicalFunction::Multiply,
        PhysicalFunction::Divide,
        PhysicalFunction::Negate,
        PhysicalFunction::Clamp,
        PhysicalFunction::Less,
        PhysicalFunction::LessOrEqual,
        PhysicalFunction::Greater,
        PhysicalFunction::GreaterOrEqual,
        PhysicalFunction::Equal,
        PhysicalFunction::NotEqual,
        PhysicalFunction::BooleanAnd,
        PhysicalFunction::BooleanOr,
        PhysicalFunction::BooleanNot,
        PhysicalFunction::StringConcat,
        PhysicalFunction::StringContains,
    ];
    for function in expected {
        assert!(
            PHYSICAL_FUNCTION_CATALOG
                .iter()
                .any(|entry| entry.function == function && entry.arity > 0)
        );
    }
    assert_eq!(
        find_physical_function("numeric_add"),
        Some(PhysicalFunction::Add)
    );
    assert_eq!(find_physical_function("does_not_exist"), None);

    let numeric = [
        PhysicalType::Int16,
        PhysicalType::Int32,
        PhysicalType::Int64,
        PhysicalType::Float32,
        PhysicalType::Float64,
    ];
    for left in &numeric {
        assert!(
            build_physical_expression(PhysicalFunction::Negate, std::slice::from_ref(left)).is_ok()
        );
        for right in &numeric {
            let supported = numeric_output(left, right).is_some();
            for function in [
                PhysicalFunction::Add,
                PhysicalFunction::Subtract,
                PhysicalFunction::Multiply,
                PhysicalFunction::Divide,
                PhysicalFunction::Less,
                PhysicalFunction::LessOrEqual,
                PhysicalFunction::Greater,
                PhysicalFunction::GreaterOrEqual,
                PhysicalFunction::Equal,
                PhysicalFunction::NotEqual,
            ] {
                assert_eq!(
                    build_physical_expression(function, &[left.clone(), right.clone()]).is_ok(),
                    supported,
                    "unexpected support for {function:?}({left:?}, {right:?})",
                );
            }

            for upper in &numeric {
                let clamp_supported = numeric_output(left, right)
                    .and_then(|pair| numeric_output(&pair, upper))
                    .is_some();
                assert_eq!(
                    build_physical_expression(
                        PhysicalFunction::Clamp,
                        &[left.clone(), right.clone(), upper.clone()],
                    )
                    .is_ok(),
                    clamp_supported,
                    "unexpected support for clamp({left:?}, {right:?}, {upper:?})",
                );
            }
        }
    }

    assert!(build_physical_expression(PhysicalFunction::BooleanNot, &[PhysicalType::Bool]).is_ok());
    assert!(
        build_physical_expression(
            PhysicalFunction::BooleanAnd,
            &[PhysicalType::Bool, PhysicalType::Bool],
        )
        .is_ok()
    );
    assert!(
        build_physical_expression(
            PhysicalFunction::StringConcat,
            &[PhysicalType::String, PhysicalType::String],
        )
        .is_ok()
    );
    assert!(build_physical_expression(PhysicalFunction::Add, &[PhysicalType::Bool]).is_err());
    assert!(
        build_physical_expression(
            PhysicalFunction::StringContains,
            &[PhysicalType::String, PhysicalType::Bool],
        )
        .is_err()
    );
}

#[test]
fn catalog_evaluates_mixed_numeric_and_fallible_clamp_through_erasure() {
    let add = build_physical_expression(
        PhysicalFunction::Add,
        &[PhysicalType::Int16, PhysicalType::Int32],
    )
    .unwrap();
    assert_eq!(add.output_type(), PhysicalType::Int32);
    let left: ArrayImpl = I16Array::from_slice(&[Some(2), None, Some(-4)]).into();
    let output = add
        .evaluate(&[
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
        ])
        .unwrap();
    assert_eq!(
        I32Array::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(7), None, Some(1)]
    );

    let clamp = build_physical_expression(
        PhysicalFunction::Clamp,
        &[
            PhysicalType::Int32,
            PhysicalType::Int16,
            PhysicalType::Int32,
        ],
    )
    .unwrap();
    let values: ArrayImpl = I32Array::from_slice(&[Some(2), None, Some(9)]).into();
    let lower: ArrayImpl = I16Array::from_slice(&[Some(0), Some(8), Some(10)]).into();
    let upper: ArrayImpl = I32Array::from_slice(&[Some(5), Some(1), Some(3)]).into();
    assert!(
        clamp
            .evaluate(&[
                ColumnViewImpl::array(&values),
                ColumnViewImpl::array(&lower),
                ColumnViewImpl::array(&upper),
            ])
            .is_err()
    );
}

#[test]
fn catalog_evaluates_nullable_boolean_logic_through_erasure() {
    let expression = build_physical_expression(
        PhysicalFunction::BooleanAnd,
        &[PhysicalType::Bool, PhysicalType::Bool],
    )
    .unwrap();
    let left: ArrayImpl = BoolArray::from_slice(&[Some(false), Some(true), None]).into();
    let right: ArrayImpl = BoolArray::from_slice(&[None, None, Some(false)]).into();
    let output = expression
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        BoolArray::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), None, Some(false)]
    );
}

#[test]
fn catalog_evaluates_strings_through_the_transactional_writer_path() {
    let expression = build_physical_expression(
        PhysicalFunction::StringConcat,
        &[PhysicalType::String, PhysicalType::String],
    )
    .unwrap();
    let left: ArrayImpl = StringArray::from_slice(&[Some("vec"), None, Some("")]).into();
    let right = ColumnViewImpl::constant(ScalarRefImpl::String("tor"), 3);
    let output = expression
        .evaluate(&[ColumnViewImpl::array(&left), right])
        .unwrap();
    let output = StringArray::try_from(output).unwrap();
    assert_eq!(output.data(), b"vectortor");
    assert_eq!(output.offsets(), &[0, 6, 6, 9]);
    assert_eq!(
        output.iter().collect::<Vec<_>>(),
        vec![Some("vector"), None, Some("tor")]
    );
}
