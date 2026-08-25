//! Learner-owned checked-operator checkpoints.
//!
//! Day 4, checkpoint 2: add checked unary and binary shells here.
// pub trait CheckedUnaryScalarFunction {
//     /* one checked scalar call: fn evaluate(&self, input: ScalarRefImpl<'_>) -> Result<Self::Output, ScalarError> */
// }
// pub trait CheckedBinaryScalarFunction {
//     /* one checked scalar call: fn evaluate(&self, left: ScalarRefImpl<'_>, right: ScalarRefImpl<'_>) -> Result<Self::Output, ScalarError> */
// }
// pub struct UnaryExpression<F> { /* function and metadata */ }
// impl<F> UnaryExpression<F> {
//     /* pub fn new(name, input_types: [PhysicalType; 1], function) -> Self */
//     /* pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, /* readable batch error of your choice */> */
// }
// pub struct CheckedBinaryExpression<F> { /* function and metadata */ }
// impl<F> CheckedBinaryExpression<F> {
//     /* pub fn new(name, input_types: [PhysicalType; 2], function) -> Self */
//     /* pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, /* readable batch error of your choice */> */
// }
//
//! Day 5, checkpoint 2: add arithmetic and numeric-comparison selection here.
// pub enum ArithmeticOperator { Add, Subtract, Multiply, Divide }
// pub enum ComparisonOperator { /* ordered comparisons */ }
// pub(crate) fn build_numeric_binary_expression(/* operator and types */) -> /* expression */;
// pub(crate) fn build_numeric_comparison_expression(/* operator and types */) -> /* expression */;
// pub(crate) fn build_numeric_neg_expression(/* input type */) -> /* expression */;
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
// impl Expression for BooleanExpression { /* metadata and evaluation */ }
