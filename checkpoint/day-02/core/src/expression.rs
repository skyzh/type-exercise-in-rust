//! Learner-owned expression checkpoints.
//!
//! Day 4, checkpoint 1: define scalar work and the first binary evaluator. Use contextual
//! `anyhow::Result` errors at the public boundary.
// pub trait BinaryScalarFunction { /* associated scalar families and one row call */ }
// pub struct I32Add;
// pub fn evaluate_binary<F: BinaryScalarFunction>(/* inputs */) -> /* output */;
//!
//! Day 4, checkpoint 2: add the fixed-width scalar adapter and whole-batch shells. Their tests
//! exercise contextual arity, type, length, and scalar-evaluation errors.
//
//! Day 9, checkpoint 1: add the object-safe runtime expression boundary here.
// pub trait Expression: Any + Send + Sync {
//     /* fn name(&self) -> &'static str */
//     /* fn arity(&self) -> usize */
//     /* fn input_types(&self) -> &[PhysicalType] */
//     /* fn output_type(&self) -> PhysicalType */
//     /* fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> */
// }
// pub type BinaryBatchKernel = /* one function pointer over a complete borrowed input batch */;
// pub struct BinaryExpression { /* physical metadata plus one whole-batch kernel */ }
// impl BinaryExpression { /* pub fn new(name, input_types, output_type, kernel) -> Self */ }
//
//! Day 9, checkpoint 3: add the builtin catalog here.
// pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>>;
// pub const BUILTIN_EXPRESSION_NAMES: &[&str];
//
//! Day 7, checkpoint 2: add one representative batch loop choice here. Select it once from
//! private raw values/validity, with Indexed inputs using the existing general fallback.
// pub enum PrimitiveLoop { /* supported dense loops plus general fallback */ }
// pub struct PrimitiveBinaryExpression<F> { /* typed i32 function plus checked input types */ }
// impl<F> PrimitiveBinaryExpression<F> {
//     pub fn new(name: &'static str, function: F) -> Self;
// }
// impl<F> PrimitiveBinaryExpression<F> {
//     pub fn evaluate_with_loop(
//         &self,
//         inputs: &[ColumnViewImpl<'_>],
//     ) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)>;
// }
//
//! Day 13: add checked Any recovery and downcast helpers to the erased boundary here.
// impl dyn Expression { /* checked Any recovery */ }
//
//! Day 14: add one static batch future here.
// pub fn evaluate_static<'a, E>(expression: &'a E, inputs: &'a [ColumnViewImpl<'a>])
//     -> impl Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a;
//
//! Day 14: add the object-safe async adapter here.
// pub type BatchFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a>>;
// pub trait AsyncExpression: Send + Sync {
//     fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
// }
// pub struct AsyncExpressionAdapter { /* one owned synchronous expression */ }
// impl AsyncExpressionAdapter {
//     pub fn new(expression: Box<dyn Expression>) -> Self;
// }
