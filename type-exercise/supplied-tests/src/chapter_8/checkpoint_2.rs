use crate::*;

fn bool_array(values: &[Option<bool>]) -> ArrayImpl {
    BoolArray::from_slice(values).into()
}

#[test]
fn evaluation_matches_the_full_truth_table() {
    let and = build_boolean_expression(BooleanOperator::And);
    let left = bool_array(&[
        Some(true),
        Some(true),
        Some(true),
        Some(false),
        Some(false),
        Some(false),
        None,
        None,
        None,
    ]);
    let right = bool_array(&[
        Some(true),
        Some(false),
        None,
        Some(true),
        Some(false),
        None,
        Some(true),
        Some(false),
        None,
    ]);
    let output = and
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![
            Some(true),
            Some(false),
            None,
            Some(false),
            Some(false),
            Some(false),
            None,
            Some(false),
            None,
        ]
    );

    let or = build_boolean_expression(BooleanOperator::Or);
    let or_left = bool_array(&[
        Some(true),
        Some(true),
        Some(true),
        Some(false),
        Some(false),
        Some(false),
        None,
        None,
        None,
    ]);
    let or_right = bool_array(&[
        Some(true),
        Some(false),
        None,
        Some(true),
        Some(false),
        None,
        Some(true),
        Some(false),
        None,
    ]);
    let output = or
        .evaluate(&[
            ColumnViewImpl::array(&or_left),
            ColumnViewImpl::array(&or_right),
        ])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![
            Some(true),
            Some(true),
            Some(true),
            Some(true),
            Some(false),
            None,
            Some(true),
            None,
            None,
        ]
    );

    let not = build_boolean_expression(BooleanOperator::Not);
    let not_input = bool_array(&[Some(true), Some(false), None]);
    let output = not.evaluate(&[ColumnViewImpl::array(&not_input)]).unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(true), None]
    );
}

#[test]
fn nullable_and_or_keep_their_sql_absorption_rules() {
    let and = build_boolean_expression(BooleanOperator::And);
    let output = and
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Bool, 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(false));

    let or = build_boolean_expression(BooleanOperator::Or);
    let output = or
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Bool, 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(true));
}

#[test]
fn strict_not_negates_non_null_rows_and_keeps_null_rows_null() {
    let strict_not = build_boolean_expression(BooleanOperator::Not);
    let input: ArrayImpl = BoolArray::from_slice(&[Some(true), Some(false), None]).into();
    let output = strict_not
        .evaluate(&[ColumnViewImpl::array(&input)])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(true), None]
    );
}

#[test]
fn not_has_arity_one_and_and_or_have_arity_two() {
    assert_eq!(build_boolean_expression(BooleanOperator::Not).arity(), 1);
    assert_eq!(build_boolean_expression(BooleanOperator::And).arity(), 2);
    assert_eq!(build_boolean_expression(BooleanOperator::Or).arity(), 2);
}

#[test]
fn metadata_and_getters_pin_the_public_contract() {
    let not = build_boolean_expression(BooleanOperator::Not);
    assert_eq!(not.operator(), BooleanOperator::Not);
    assert_eq!(not.input_types(), &[PhysicalType::Bool]);
    assert_eq!(not.output_type(), PhysicalType::Bool);

    let and = build_boolean_expression(BooleanOperator::And);
    assert_eq!(and.operator(), BooleanOperator::And);
    assert_eq!(and.input_types(), &[PhysicalType::Bool, PhysicalType::Bool]);
    assert_eq!(and.output_type(), PhysicalType::Bool);

    let or = build_boolean_expression(BooleanOperator::Or);
    assert_eq!(or.operator(), BooleanOperator::Or);
}

#[test]
fn each_boolean_operator_preserves_its_public_result() {
    let and = build_boolean_expression(BooleanOperator::And);
    let output = and
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(false));

    let or = build_boolean_expression(BooleanOperator::Or);
    let output = or
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(true));

    let not = build_boolean_expression(BooleanOperator::Not);
    let output = not
        .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1)])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(false));
}

#[test]
fn boolean_expressions_reject_wrong_arity_types_and_lengths() {
    let and = build_boolean_expression(BooleanOperator::And);
    let not = build_boolean_expression(BooleanOperator::Not);

    assert!(
        and.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
        ])
        .is_err()
    );
    assert!(
        and.evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1)])
            .is_err()
    );
    assert!(not.evaluate(&[]).is_err());
    assert!(
        not.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
        ])
        .is_err()
    );

    assert!(
        and.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
        ])
        .is_err()
    );
    // The second input's physical type must be checked too, not only the first.
    assert!(
        and.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ])
        .is_err()
    );
    // Type validation precedes length validation: a wrong second type with a
    // mismatched length still fails closed.
    assert!(
        and.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
        ])
        .is_err()
    );

    assert!(
        and.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 3),
        ])
        .is_err()
    );
}
