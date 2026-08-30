use crate::{
    Array, ArrayImpl, BoolArray, BooleanExpression, BooleanOperator, ColumnViewImpl, PhysicalType,
    ScalarRefImpl, build_boolean_expression,
};

#[derive(Clone, Copy)]
struct BooleanTruthRow {
    operator: BooleanOperator,
    left: Option<bool>,
    right: Option<bool>,
    result: Option<bool>,
}

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
    for row in EXPECTED_TRUTH_TABLE {
        let expression = build_boolean_expression(row.operator);
        let left = match row.left {
            Some(value) => ColumnViewImpl::constant(ScalarRefImpl::Bool(value), 1),
            None => ColumnViewImpl::null(PhysicalType::Bool, 1),
        };
        let inputs = if row.operator == BooleanOperator::Not {
            vec![left]
        } else {
            let right = match row.right {
                Some(value) => ColumnViewImpl::constant(ScalarRefImpl::Bool(value), 1),
                None => ColumnViewImpl::null(PhysicalType::Bool, 1),
            };
            vec![left, right]
        };
        let output = expression.evaluate(&inputs).unwrap();
        assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), row.result);
    }
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
    let and = BooleanExpression::new(BooleanOperator::And);
    let output = and
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Bool, 1),
            ColumnViewImpl::constant(ScalarRefImpl::Bool(false), 1),
        ])
        .unwrap();
    assert_eq!(<&BoolArray>::try_from(&output).unwrap().get(0), Some(false));

    let or = BooleanExpression::new(BooleanOperator::Or);
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
    let strict_not = BooleanExpression::new(BooleanOperator::Not);
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

    let or = BooleanExpression::new(BooleanOperator::Or);
    assert_eq!(or.operator(), BooleanOperator::Or);
}

#[test]
fn operation_selection_stays_outside_the_shared_row_loops() {
    let source = include_str!("../boolean.rs");
    assert!(!source.contains("NullEvaluationPolicy"));
    assert!(source.contains("match self.operator"));
    let core = include_str!("../expression.rs");
    assert!(!core.contains("BooleanOperator"));
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
