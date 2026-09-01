#[path = "../../expr/src/boolean.rs"]
mod boolean;
use boolean::{and, not, or};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TruthOperation {
    And,
    Or,
    Not,
}

const EXPECTED_TRUTH_TABLE: &[(TruthOperation, Option<bool>, Option<bool>, Option<bool>)] = &[
    (TruthOperation::And, Some(true), Some(true), Some(true)),
    (TruthOperation::And, Some(true), Some(false), Some(false)),
    (TruthOperation::And, Some(true), None, None),
    (TruthOperation::And, Some(false), Some(true), Some(false)),
    (TruthOperation::And, Some(false), Some(false), Some(false)),
    (TruthOperation::And, Some(false), None, Some(false)),
    (TruthOperation::And, None, Some(true), None),
    (TruthOperation::And, None, Some(false), Some(false)),
    (TruthOperation::And, None, None, None),
    (TruthOperation::Or, Some(true), Some(true), Some(true)),
    (TruthOperation::Or, Some(true), Some(false), Some(true)),
    (TruthOperation::Or, Some(true), None, Some(true)),
    (TruthOperation::Or, Some(false), Some(true), Some(true)),
    (TruthOperation::Or, Some(false), Some(false), Some(false)),
    (TruthOperation::Or, Some(false), None, None),
    (TruthOperation::Or, None, Some(true), Some(true)),
    (TruthOperation::Or, None, Some(false), None),
    (TruthOperation::Or, None, None, None),
    (TruthOperation::Not, Some(true), None, Some(false)),
    (TruthOperation::Not, Some(false), None, Some(true)),
    (TruthOperation::Not, None, None, None),
];

#[test]
fn checkpoint_1_scalar_functions_match_all_sql_truth_rows() {
    assert_eq!(EXPECTED_TRUTH_TABLE.len(), 21);
    for &(operation, left, right, expected) in EXPECTED_TRUTH_TABLE {
        let actual = match operation {
            TruthOperation::And => and(left, right),
            TruthOperation::Or => or(left, right),
            TruthOperation::Not => left.map(not),
        };
        assert_eq!(actual, expected, "{operation:?}({left:?}, {right:?})");
    }
}
