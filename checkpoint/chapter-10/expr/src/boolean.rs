//! SQL three-valued Boolean scalar operations.

use crate::{
    ArrayImpl, BatchExpression, ColumnViewImpl, ComparisonOperator, PhysicalType,
    auto_vectorize_binary, auto_vectorize_unary, evaluate_nullable_binary,
};

/// One three-valued Boolean operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanOperator {
    And,
    Or,
    Not,
}

pub(crate) fn not(value: bool) -> bool {
    !value
}

pub(crate) fn and(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

pub(crate) fn or(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

fn evaluate_boolean_not(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_unary::<bool, bool, _>(inputs[0].clone(), not)
}

fn evaluate_boolean_and(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    evaluate_nullable_binary::<bool, bool, bool, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        |left, right| Ok(and(left, right)),
    )
}

fn evaluate_boolean_or(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    evaluate_nullable_binary::<bool, bool, bool, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        |left, right| Ok(or(left, right)),
    )
}

pub(crate) fn build_boolean_not_expression(name: &'static str) -> BatchExpression<1> {
    BatchExpression::new(
        name,
        [PhysicalType::Bool],
        PhysicalType::Bool,
        evaluate_boolean_not,
    )
}

pub(crate) fn build_boolean_binary_expression(
    name: &'static str,
    operator: BooleanOperator,
) -> BatchExpression<2> {
    let kernel = match operator {
        BooleanOperator::And => evaluate_boolean_and,
        BooleanOperator::Or => evaluate_boolean_or,
        BooleanOperator::Not => unreachable!("unary Boolean operator in binary builder"),
    };
    BatchExpression::new(
        name,
        [PhysicalType::Bool, PhysicalType::Bool],
        PhysicalType::Bool,
        kernel,
    )
}

fn evaluate_bool_equal(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_binary::<bool, bool, bool, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        crate::numeric::equal,
    )
}

fn evaluate_bool_not_equal(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_binary::<bool, bool, bool, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        crate::numeric::not_equal,
    )
}

pub(crate) fn build_bool_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
) -> BatchExpression<2> {
    let kernel = match operator {
        ComparisonOperator::Equal => evaluate_bool_equal,
        ComparisonOperator::NotEqual => evaluate_bool_not_equal,
        _ => unreachable!("ordered boolean comparison"),
    };
    BatchExpression::new(
        name,
        [PhysicalType::Bool, PhysicalType::Bool],
        PhysicalType::Bool,
        kernel,
    )
}
