//! Learner-owned expression checkpoints.
//!
//! Day 4, checkpoint 1: define scalar work and the first binary evaluator here.
// pub trait BinaryScalarFunction { /* associated scalar families and one row call */ }
// pub struct I32Add;
// pub fn evaluate_binary<F: BinaryScalarFunction>(/* inputs */) -> /* output */;
//
//! Day 4, checkpoint 2: define the checked scalar error used by the checked shells here.
// pub enum ScalarError { DivisionByZero, /* more checked failures arrive with their days */ }
//
//! Day 6, checkpoint 1: add one structured batch-evaluation error here.
// pub enum ExpressionError { /* arity, type, length, and scalar failures */ }
//
//! Day 8, checkpoint 1: add the object-safe runtime expression boundary here.
// pub trait Expression: Any + Send + Sync {
//     /* fn name(&self) -> &'static str */
//     /* fn arity(&self) -> usize */
//     /* fn input_types(&self) -> &[PhysicalType] */
//     /* fn output_type(&self) -> PhysicalType */
//     /* fn output_nullability(&self, inputs: &[Nullability]) -> Nullability */
//     /* fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> */
// }
// pub struct BinaryExpression<F> { /* typed function plus runtime metadata */ }
// impl<F> BinaryExpression<F> { /* pub fn new(name, function) -> Self */ }
//
//! Day 8, checkpoint 3: add the builtin catalog here.
// pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>>;
// pub const BUILTIN_EXPRESSION_NAMES: &[&str];
//
//! Day 10, checkpoint 1: add one representative batch loop choice here. Select it only from checked physical `Nullability` metadata while keeping one primitive array representation.
// pub enum PrimitiveLoop { /* supported dense loops plus general fallback */ }
// pub struct PrimitiveBinaryExpression<F> { /* typed i32 function plus runtime metadata */ }
// impl<F> PrimitiveBinaryExpression<F> {
//     pub fn new(name: &'static str, function: F) -> Self;
// }
// impl<F> PrimitiveBinaryExpression<F> {
//     pub fn evaluate_with_loop(
//         &self,
//         inputs: &[ColumnViewImpl<'_>],
//     ) -> Result<(ArrayImpl, PrimitiveLoop), ExpressionError>;
// }
//
//! Day 12, checkpoint 2: add checked Any recovery and downcast helpers to the erased boundary here.
// impl dyn Expression { /* checked Any recovery */ }
//
//! Day 13, checkpoint 1: add one static batch future here.
// pub fn evaluate_static<'a, E>(expression: &'a E, inputs: &'a [ColumnViewImpl<'a>])
//     -> impl Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a;
//
//! Day 13, checkpoint 2: add the object-safe async adapter here.
// pub type BatchFuture<'a> = Pin<Box<dyn Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a>>;
// pub trait AsyncExpression: Send + Sync {
//     fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
// }
// pub struct AsyncExpressionAdapter { /* one owned synchronous expression */ }
// impl AsyncExpressionAdapter {
//     pub fn new(expression: Box<dyn Expression>) -> Self;
// }
