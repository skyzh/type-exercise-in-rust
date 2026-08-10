#![forbid(unsafe_code)]

mod array;
mod data_type;
mod decimal;
mod physical_type;
mod scalar;
mod variant_catalog;

// Day 3, checkpoint 1: uncomment after implementing `src/column.rs`.
// mod column;
// pub use column::{ColumnView, ColumnViewImpl};
// Day 4, checkpoints 1–2: uncomment after implementing the expression/operator skeletons.
// mod expression;
// mod operators;
// pub use expression::{BinaryScalarFunction, I32Add, ScalarError, evaluate_binary};
// pub use operators::{
//     CheckedBinaryExpression, CheckedBinaryScalarFunction, CheckedUnaryScalarFunction,
//     UnaryExpression,
// };
// Day 5, checkpoint 1: uncomment after implementing numeric promotion.
// mod promotion;
// pub use promotion::{NUMERIC_PROMOTIONS, NumericPromotion, promote_numeric};
// Day 5, checkpoint 2: extend the operators re-export with the selection enums.
// pub use operators::{ArithmeticOperator, ComparisonOperator};
// Day 6, checkpoint 1: extend the expression and operators re-exports with the shared error and validator.
// pub use expression::ExpressionError;
// pub use operators::validate_expression_inputs;
// Day 7, checkpoint 1: uncomment after implementing three-valued Boolean logic.
// mod boolean_logic;
// pub use boolean_logic::{
//     BOOLEAN_TRUTH_TABLE, BooleanExpression, BooleanOperator, BooleanTruthRow,
//     NullEvaluationPolicy, build_boolean_expression,
// };
// Day 8, checkpoint 1: extend the expression re-exports with the erased boundary and catalog.
// pub use expression::{
//     BinaryExpression, BUILTIN_EXPRESSION_NAMES, Expression, build_builtin_expression,
// };
// Day 9, checkpoint 1: uncomment after implementing the binder and registry.
// mod binder;

#[cfg(test)]
mod tests;

pub use array::*;
pub use data_type::*;
pub use decimal::*;
pub use physical_type::*;
pub use scalar::*;
