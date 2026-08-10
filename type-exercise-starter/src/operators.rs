//! Learner-owned checked-operator checkpoints.
//!
//! Day 4, checkpoint 2: add checked unary and binary shells here.
// pub trait CheckedUnaryScalarFunction { /* one checked scalar call */ }
// pub trait CheckedBinaryScalarFunction { /* one checked scalar call */ }
// pub struct UnaryExpression<F> { /* function and metadata */ }
// pub struct CheckedBinaryExpression<F> { /* function and metadata */ }
//
//! Day 5, checkpoint 2: add arithmetic and numeric-comparison selection here.
// pub enum ArithmeticOperator { Add, Subtract, Multiply, Divide }
// pub enum ComparisonOperator { /* ordered comparisons */ }
// pub(crate) fn build_numeric_binary_expression(/* operator and types */) -> /* expression */;
// pub(crate) fn build_numeric_comparison_expression(/* operator and types */) -> /* expression */;
//
//! Day 6, checkpoint 1: share arity, physical-type, and length validation here.
// pub fn validate_expression_inputs(/* expected types and inputs */) -> Result<usize, ExpressionError>;
//
//! Day 6, checkpoint 2: add the checked ternary shell here.
// pub trait CheckedTernaryScalarFunction { /* one checked scalar call */ }
// pub struct TernaryExpression<F> { /* function and metadata */ }
//
//! Day 6, checkpoint 3: make the ternary path observable with clamp.
// pub(crate) fn build_numeric_clamp_expression(/* physical type */) -> /* expression */;
//
//! Day 8, checkpoint 2: implement the erased Expression boundary for each checked shell here.
// impl<F> Expression for UnaryExpression<F> { /* metadata and evaluation */ }
// impl<F> Expression for CheckedBinaryExpression<F> { /* metadata and evaluation */ }
// impl<F> Expression for TernaryExpression<F> { /* metadata and evaluation */ }
