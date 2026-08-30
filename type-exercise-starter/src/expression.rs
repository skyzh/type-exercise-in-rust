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
//! Day 5, checkpoint 2: pair selected binary metadata with one generated batch adapter.
// pub type BinaryBatchKernel = /* one function pointer over a complete borrowed input batch */;
// pub struct BinaryExpression { /* physical metadata plus one whole-batch kernel */ }
// impl BinaryExpression { /* pub fn new(name, input_types, output_type, kernel) -> Self */ }
// /* Keep any constructor used only to attach expression names to row errors crate-private. */
//
//! Day 8, checkpoint 1: add the object-safe runtime expression boundary here.
// pub trait Expression: Any + Send + Sync {
//     /* fn name(&self) -> &'static str */
//     /* fn arity(&self) -> usize */
//     /* fn input_types(&self) -> &[PhysicalType] */
//     /* fn output_type(&self) -> PhysicalType */
//     /* fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> */
// }
//! Day 8, checkpoint 3: add the builtin catalog here.
// pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>>;
// pub const BUILTIN_EXPRESSION_NAMES: &[&str];
//
//! Day 10, checkpoint 1: add one representative batch loop choice here. Select a dense loop only
//! when both checked input views carry `Nullability::NonNull`; read the same primitive arrays'
//! `values()` without adding another array representation.
//     /* fn output_nullability(&self, inputs: &[Nullability]) -> Nullability */
// pub type BinaryLoopKernel = /* whole-batch kernel plus selected PrimitiveLoop */;
// impl BinaryExpression { /* pub fn new_with_loop(..., kernel, loop_kernel) -> Self */ }
// pub enum PrimitiveLoop { /* supported dense loops plus general fallback */ }
// pub struct PrimitiveBinaryExpression<F> { /* typed i32 function plus runtime metadata */ }
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
//! Day 12, checkpoint 2: add checked Any recovery and downcast helpers to the erased boundary here.
// impl dyn Expression { /* checked Any recovery */ }
//
//! Day 13, checkpoint 1: add one static batch future here.
// pub fn evaluate_static<'a, E>(expression: &'a E, inputs: &'a [ColumnViewImpl<'a>])
//     -> impl Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a;
//
//! Day 13, checkpoint 2: add the object-safe async adapter here.
// pub type BatchFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a>>;
// pub trait AsyncExpression: Send + Sync {
//     fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
// }
// pub struct AsyncExpressionAdapter { /* one owned synchronous expression */ }
// impl AsyncExpressionAdapter {
//     pub fn new(expression: Box<dyn Expression>) -> Self;
// }
