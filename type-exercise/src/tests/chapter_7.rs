use crate::{
    Array, ArrayImpl, BOOLEAN_TRUTH_TABLE, BoolArray, BooleanExpression, BooleanOperator,
    BooleanTruthRow, ColumnViewImpl, NullEvaluationPolicy, PhysicalType, ScalarRefImpl,
    build_boolean_expression,
};

const EXPECTED_TRUTH_TABLE: &[BooleanTruthRow] = &[
    // AND
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: Some(true),
        right: Some(true),
        result: Some(true),
    },
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: Some(true),
        right: Some(false),
        result: Some(false),
    },
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: Some(true),
        right: None,
        result: None,
    },
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: Some(false),
        right: Some(true),
        result: Some(false),
    },
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: Some(false),
        right: Some(false),
        result: Some(false),
    },
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: Some(false),
        right: None,
        result: Some(false),
    },
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: None,
        right: Some(true),
        result: None,
    },
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: None,
        right: Some(false),
        result: Some(false),
    },
    BooleanTruthRow {
        operator: BooleanOperator::And,
        left: None,
        right: None,
        result: None,
    },
    // OR
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: Some(true),
        right: Some(true),
        result: Some(true),
    },
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: Some(true),
        right: Some(false),
        result: Some(true),
    },
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: Some(true),
        right: None,
        result: Some(true),
    },
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: Some(false),
        right: Some(true),
        result: Some(true),
    },
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: Some(false),
        right: Some(false),
        result: Some(false),
    },
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: Some(false),
        right: None,
        result: None,
    },
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: None,
        right: Some(true),
        result: Some(true),
    },
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: None,
        right: Some(false),
        result: None,
    },
    BooleanTruthRow {
        operator: BooleanOperator::Or,
        left: None,
        right: None,
        result: None,
    },
    // NOT
    BooleanTruthRow {
        operator: BooleanOperator::Not,
        left: Some(true),
        right: None,
        result: Some(false),
    },
    BooleanTruthRow {
        operator: BooleanOperator::Not,
        left: Some(false),
        right: None,
        result: Some(true),
    },
    BooleanTruthRow {
        operator: BooleanOperator::Not,
        left: None,
        right: None,
        result: None,
    },
];

fn bool_array(values: &[Option<bool>]) -> ArrayImpl {
    BoolArray::from_slice(values).into()
}

#[test]
fn truth_table_matches_sql_three_valued_semantics() {
    assert_eq!(EXPECTED_TRUTH_TABLE.len(), 21);
    assert_eq!(BOOLEAN_TRUTH_TABLE, EXPECTED_TRUTH_TABLE);
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
fn strict_policy_short_circuits_before_the_truth_table() {
    let strict_and = BooleanExpression::new(BooleanOperator::And, NullEvaluationPolicy::Strict);
    let output = strict_and
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Bool, 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), None);

    let sql_and = BooleanExpression::new(BooleanOperator::And, NullEvaluationPolicy::NonStrict);
    let output = sql_and
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Bool, 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
        ])
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

    assert!(
        and.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 3),
        ])
        .is_err()
    );
}
