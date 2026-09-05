use crate::*;

#[test]
fn binds_exact_and_losslessly_widened_numeric_calls() {
    let exact = bind_logical_call(LogicalCall::new(
        "+",
        [DataType::Integer, DataType::Integer],
    ))
    .unwrap();
    assert_eq!(exact.logical_call().name(), "+");
    assert_eq!(
        exact.logical_call().input_types(),
        &[DataType::Integer, DataType::Integer]
    );
    assert_eq!(exact.output_type(), &DataType::Integer);
    assert_eq!(exact.physical_expression().name(), "numeric_add");
    assert_eq!(
        exact.physical_expression().input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );

    let widened =
        bind_logical_call(LogicalCall::new("+", [DataType::Integer, DataType::Real])).unwrap();
    assert_eq!(widened.output_type(), &DataType::Double);
    assert_eq!(
        widened.physical_expression().input_types(),
        &[PhysicalType::Int32, PhysicalType::Float32]
    );
    assert_eq!(
        widened.physical_expression().output_type(),
        PhysicalType::Float64
    );
}

#[test]
fn evaluates_the_returned_erased_numeric_expression() {
    let expression = bind_logical_call(LogicalCall::new(
        "+",
        [DataType::SmallInt, DataType::Integer],
    ))
    .unwrap()
    .into_physical_expression();
    let left: ArrayImpl = I16Array::from_slice(&[Some(2), None, Some(-4)]).into();
    let output = expression
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
}

#[test]
fn binds_char_and_varchar_to_string_concat() {
    let bound = bind_logical_call(LogicalCall::new(
        "concat",
        [DataType::Char { width: 3 }, DataType::Varchar],
    ))
    .unwrap();
    assert_eq!(bound.output_type(), &DataType::Varchar);
    assert_eq!(
        bound.physical_expression().input_types(),
        &[PhysicalType::String, PhysicalType::String]
    );

    let left: ArrayImpl = StringArray::from_slice(&[Some("vec"), None, Some("")]).into();
    let output = bound
        .evaluate(&[
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::String("tor"), 3),
        ])
        .unwrap();
    assert_eq!(
        StringArray::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some("vector"), None, Some("tor")]
    );
}

#[test]
fn binds_boolean_and_ternary_numeric_calls() {
    let boolean = bind_logical_call(LogicalCall::new(
        "boolean_and",
        [DataType::Boolean, DataType::Boolean],
    ))
    .unwrap();
    assert_eq!(boolean.output_type(), &DataType::Boolean);
    assert_eq!(
        boolean.physical_expression().output_type(),
        PhysicalType::Bool
    );

    let clamp = bind_logical_call(LogicalCall::new(
        "clamp",
        [DataType::SmallInt, DataType::Integer, DataType::BigInt],
    ))
    .unwrap();
    assert_eq!(clamp.output_type(), &DataType::BigInt);
    assert_eq!(
        clamp.physical_expression().input_types(),
        &[
            PhysicalType::Int16,
            PhysicalType::Int32,
            PhysicalType::Int64
        ]
    );
}

#[test]
fn rejects_invalid_logical_and_claimed_physical_signatures() {
    assert!(
        bind_logical_call(LogicalCall::new(
            "missing",
            [DataType::Integer, DataType::Integer]
        ))
        .is_err()
    );
    assert!(bind_logical_call(LogicalCall::new("+", [DataType::Integer])).is_err());
    assert!(
        bind_logical_call(LogicalCall::new("+", [DataType::BigInt, DataType::Double])).is_err()
    );
    assert!(
        bind_logical_call(LogicalCall::new(
            "concat",
            [DataType::Varchar, DataType::Boolean]
        ))
        .is_err()
    );

    let expression =
        build_physical_expression(PhysicalFunction::BooleanNot, &[PhysicalType::Bool]).unwrap();
    assert!(
        BoundExpression::new(
            LogicalCall::new("claimed_integer", [DataType::Integer]),
            DataType::Integer,
            expression,
        )
        .is_err()
    );
}
