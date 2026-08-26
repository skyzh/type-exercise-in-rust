#![forbid(unsafe_code)]

mod array;
mod physical_type;
mod scalar;
mod variant_catalog;

// Day 2, checkpoints 3–4: uncomment these modules and exports after implementing logical types
// and Decimal metadata/storage in `src/data_type.rs`, `src/decimal.rs`, and
// `src/array/decimal_array.rs`.
// mod data_type;
// mod decimal;
// pub use data_type::*;
// pub use decimal::*;

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
// Day 10, checkpoint 1: extend the expression re-export with `PrimitiveBinaryExpression` and
// `PrimitiveLoop`; `Nullability` is exported beside `PhysicalType`.
// Day 9, checkpoint 1: uncomment after implementing the binder and registry.
// mod binder;
// pub use binder::{BindError, BoundExpression, FunctionRegistry};
// Day 13, checkpoints 1–3: extend the expression re-export with `AsyncExpression`,
// `AsyncExpressionAdapter`, `BatchFuture`, and `evaluate_static`; `BoundExpression` implements the
// async adapter through the enabled binder module.

// Day 11, checkpoints 1–3: the active `pub use array::*` below exports `ListArray`, `ListError`,
// `ListScalar`, and `ListScalarRef` after their module is enabled. Replace the Day 3 column
// re-export with the extended line below; `try_as_list` remains a `ColumnViewImpl` method.
// pub use column::{ColumnView, ColumnViewImpl, ListColumnView};

#[cfg(test)]
mod tests;

pub use array::*;
pub use physical_type::*;
pub use scalar::*;
