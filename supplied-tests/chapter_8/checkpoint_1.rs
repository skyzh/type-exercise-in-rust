use crate::{
    Array, ArrayImpl, BoolArray, BooleanExpression, BooleanOperator, ColumnViewImpl,
    PhysicalType, ScalarRefImpl, build_boolean_expression,
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
        assert_eq!(
            <&BoolArray>::try_from(&output).unwrap().get(0),
            row.result
        );
    }
}
