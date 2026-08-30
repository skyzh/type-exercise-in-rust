//! SQL three-valued Boolean scalar operations.

use crate::{ArrayImpl, ColumnViewImpl, Expression, PhysicalType};

/// One three-valued Boolean operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanOperator {
    And,
    Or,
    Not,
}

fn boolean_and(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

fn boolean_or(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

/// A checked three-valued Boolean expression over `Boolean` columns.
#[derive(Clone, Debug)]
pub struct BooleanExpression {
    operator: BooleanOperator,
}

impl BooleanExpression {
    pub fn new(operator: BooleanOperator) -> Self {
        Self { operator }
    }

    pub fn operator(&self) -> BooleanOperator {
        self.operator
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
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        crate::validate_expression_inputs(inputs, self.input_types())?;
        match self.operator {
            BooleanOperator::Not => {
                crate::evaluate_unary::<bool, bool, _>(inputs[0].clone(), |value| !value)
            }
            BooleanOperator::And => crate::evaluate_nullable_binary::<bool, bool, bool, _>(
                inputs[0].clone(),
                inputs[1].clone(),
                |left, right| Ok(boolean_and(left, right)),
            ),
            BooleanOperator::Or => crate::evaluate_nullable_binary::<bool, bool, bool, _>(
                inputs[0].clone(),
                inputs[1].clone(),
                |left, right| Ok(boolean_or(left, right)),
            ),
        }
    }
}

/// Build the course's three-valued Boolean expression with SQL null semantics.
pub fn build_boolean_expression(operator: BooleanOperator) -> BooleanExpression {
    BooleanExpression::new(operator)
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
        const BOOL_PAIR: [PhysicalType; 2] = [PhysicalType::Bool, PhysicalType::Bool];
        const BOOL_SINGLE: [PhysicalType; 1] = [PhysicalType::Bool];
        match self.operator {
            BooleanOperator::And | BooleanOperator::Or => &BOOL_PAIR,
            BooleanOperator::Not => &BOOL_SINGLE,
        }
    }

    fn output_type(&self) -> PhysicalType {
        PhysicalType::Bool
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate(inputs)
    }
}
