use crate::*;

#[test]
fn generates_the_complete_builtin_catalog() {
    assert!(BUILTIN_EXPRESSION_NAMES.contains(&"i32_add"));
    for name in BUILTIN_EXPRESSION_NAMES {
        assert_eq!(build_builtin_expression(name).unwrap().name(), *name);
    }
    assert!(build_builtin_expression("add").is_none());
    assert!(build_builtin_expression("missing").is_none());
}

#[test]
fn i32_builtin_preserves_wrapping_and_null_semantics() {
    let expression = build_builtin_expression("i32_add").unwrap();
    let left: ArrayImpl = I32Array::from_slice(&[Some(i32::MAX), None, Some(7)]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
        ])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(i32::MIN), None, Some(8)]
    );
}

#[test]
fn boolean_expressions_delegate_through_the_erased_boundary() {
    let and: Box<dyn Expression> = Box::new(build_boolean_expression(BooleanOperator::And));
    assert_eq!(and.name(), "boolean_and");
    assert_eq!(and.arity(), 2);
    assert_eq!(and.input_types(), &[PhysicalType::Bool, PhysicalType::Bool]);
    assert_eq!(and.output_type(), PhysicalType::Bool);

    let left: ArrayImpl = BoolArray::from_slice(&[Some(true), Some(false), None]).into();
    let right: ArrayImpl = BoolArray::from_slice(&[Some(false), Some(true), Some(false)]).into();
    let result = and
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&result)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(false), Some(false)]
    );

    let not: Box<dyn Expression> = Box::new(build_boolean_expression(BooleanOperator::Not));
    assert_eq!(not.name(), "boolean_not");
    assert_eq!(not.arity(), 1);
    assert_eq!(not.input_types(), &[PhysicalType::Bool]);
    let result = not.evaluate(&[ColumnViewImpl::array(&left)]).unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&result)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(true), None]
    );
}
