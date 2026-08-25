//! Day 7, checkpoint 1: three-valued Boolean logic.

use crate::{
    ArrayBuilder, ArrayImpl, BoolArrayBuilder, ColumnView, ColumnViewImpl, Expression,
    ExpressionError, PhysicalType, Scalar, TypeMismatch,
};

/// How null inputs reach the scalar Boolean function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NullEvaluationPolicy {
    /// A null input produces a null row without calling the scalar function.
    Strict,
    /// Null inputs are passed to the scalar function, which applies the
    /// three-valued truth table (SQL semantics).
    NonStrict,
}

/// One three-valued Boolean operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanOperator {
    And,
    Or,
    Not,
}

/// One row of the three-valued Boolean truth table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BooleanTruthRow {
    pub operator: BooleanOperator,
    pub left: Option<bool>,
    pub right: Option<bool>,
    pub result: Option<bool>,
}

/// The required nullable-Boolean rows: nine `AND` rows, nine `OR` rows, and
/// three `NOT` rows (the right operand is unused for `NOT`).
pub const BOOLEAN_TRUTH_TABLE: &[BooleanTruthRow] = &[
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

/// Apply one three-valued Boolean operator.
pub(crate) fn apply_boolean(
    operator: BooleanOperator,
    left: Option<bool>,
    right: Option<bool>,
) -> Option<bool> {
    match operator {
        BooleanOperator::And => match (left, right) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None,
        },
        BooleanOperator::Or => match (left, right) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        },
        BooleanOperator::Not => left.map(|value| !value),
    }
}

/// A checked three-valued Boolean expression over `Boolean` columns.
#[derive(Clone, Debug)]
pub struct BooleanExpression {
    operator: BooleanOperator,
    policy: NullEvaluationPolicy,
}

impl BooleanExpression {
    pub fn new(operator: BooleanOperator, policy: NullEvaluationPolicy) -> Self {
        Self { operator, policy }
    }

    pub fn operator(&self) -> BooleanOperator {
        self.operator
    }

    pub fn policy(&self) -> NullEvaluationPolicy {
        self.policy
    }

    /// The number of Boolean inputs this operator consumes.
    pub fn arity(&self) -> usize {
        match self.operator {
            BooleanOperator::And | BooleanOperator::Or => 2,
            BooleanOperator::Not => 1,
        }
    }

    /// The physical input types in argument order.
    pub fn input_types(&self) -> &[PhysicalType] {
        const BOOL_PAIR: [PhysicalType; 2] = [PhysicalType::Bool, PhysicalType::Bool];
        const BOOL_SINGLE: [PhysicalType; 1] = [PhysicalType::Bool];
        match self.operator {
            BooleanOperator::And | BooleanOperator::Or => &BOOL_PAIR,
            BooleanOperator::Not => &BOOL_SINGLE,
        }
    }

    /// The physical output type, always `Boolean`.
    pub fn output_type(&self) -> PhysicalType {
        PhysicalType::Bool
    }

    /// Strict concrete evaluation of one three-valued Boolean operator.
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let arity = match self.operator {
            BooleanOperator::And | BooleanOperator::Or => 2,
            BooleanOperator::Not => 1,
        };
        if inputs.len() != arity {
            return Err(ExpressionError::InputArityMismatch {
                expected: arity,
                actual: inputs.len(),
            });
        }
        for input in inputs {
            if input.physical_type() != PhysicalType::Bool {
                return Err(TypeMismatch {
                    expected: PhysicalType::Bool,
                    actual: input.physical_type(),
                }
                .into());
            }
        }
        // Typed view conversion happens before length validation, so a wrong
        // later input type can never be reported as a length-first failure
        // even if the explicit type loop above were narrowed.
        let left_view = ColumnView::<bool>::try_from(inputs[0].clone())?;
        let right_view = inputs
            .get(1)
            .cloned()
            .map(ColumnView::<bool>::try_from)
            .transpose()?;
        let len = inputs.first().map_or(0, ColumnViewImpl::len);
        for (input_index, input) in inputs.iter().enumerate().skip(1) {
            if input.len() != len {
                return Err(ExpressionError::InputLengthMismatch {
                    expected: len,
                    actual: input.len(),
                    input_index,
                });
            }
        }

        let mut output = BoolArrayBuilder::with_capacity(len);
        for row in 0..len {
            let left = left_view.get(row);
            let right = right_view.as_ref().and_then(|view| view.get(row));
            let value = match self.policy {
                NullEvaluationPolicy::Strict => match self.operator {
                    BooleanOperator::Not => {
                        left.and_then(|value| apply_boolean(self.operator, Some(value), None))
                    }
                    BooleanOperator::And | BooleanOperator::Or => match (left, right) {
                        (Some(left), Some(right)) => {
                            apply_boolean(self.operator, Some(left), Some(right))
                        }
                        _ => None,
                    },
                },
                NullEvaluationPolicy::NonStrict => apply_boolean(self.operator, left, right),
            };
            output.push(value.as_ref().map(Scalar::as_scalar_ref));
        }
        Ok(output.finish().into())
    }
}

/// Build the course's three-valued Boolean expression with SQL null semantics.
pub fn build_boolean_expression(operator: BooleanOperator) -> BooleanExpression {
    BooleanExpression::new(operator, NullEvaluationPolicy::NonStrict)
}

impl Expression for BooleanExpression {
    fn name(&self) -> &'static str {
        match self.operator {
            BooleanOperator::And => "boolean_and",
            BooleanOperator::Or => "boolean_or",
            BooleanOperator::Not => "boolean_not",
        }
    }

    fn input_types(&self) -> &[PhysicalType] {
        BooleanExpression::input_types(self)
    }

    fn output_type(&self) -> PhysicalType {
        PhysicalType::Bool
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        self.evaluate(inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColumnViewImpl, ScalarRefImpl};

    /// Reference-only internal regression: physical-type validation precedes
    /// length validation even for a later input, so a first-input-only type
    /// loop cannot turn a wrong later type into a length-first failure.
    #[test]
    fn type_validation_precedes_length_validation_for_later_inputs() {
        let and = build_boolean_expression(BooleanOperator::And);
        let err = and
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
            ])
            .unwrap_err();
        assert!(
            matches!(err, ExpressionError::TypeMismatch(_)),
            "expected a type-category error before any length check, got {err:?}"
        );
    }
}
