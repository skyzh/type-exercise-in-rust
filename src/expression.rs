//! Learner-owned expression checkpoints.
//!
//! Day 4, checkpoint 1: define scalar work, the first binary evaluator, and the completed public
//! error types required by the library compile gate here.
// pub trait BinaryScalarFunction { /* associated scalar families and one row call */ }
// pub struct I32Add;
// pub fn evaluate_binary<F: BinaryScalarFunction>(/* inputs */) -> /* output */;
//
// pub enum ScalarError { DivisionByZero, DivisionOverflow }
// pub enum ExpressionError {
//     TypeMismatch(TypeMismatch),
//     InputArityMismatch { expected: usize, actual: usize },
//     InputLengthMismatch { expected: usize, actual: usize, input_index: usize },
//     ScalarEvaluation { function: &'static str, row: usize, error: ScalarError },
// }
//! The exact `Display` sentences remain your choice.
//!
//! Day 4, checkpoint 2: add the checked scalar hooks and shells. Their tests begin exercising the
//! completed error contract's arity and scalar-evaluation cases.
//
//! Day 8, checkpoint 1: add the object-safe runtime expression boundary here.
// pub trait Expression: Any + Send + Sync {
//     /* fn name(&self) -> &'static str */
//     /* fn arity(&self) -> usize */
//     /* fn input_types(&self) -> &[PhysicalType] */
//     /* fn output_type(&self) -> PhysicalType */
//     /* fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> */
// }
// pub type BinaryBatchKernel = /* one function pointer over a complete borrowed input batch */;
// pub struct BinaryExpression { /* physical metadata plus one whole-batch kernel */ }
// impl BinaryExpression { /* pub fn new(name, input_types, output_type, kernel) -> Self */ }
//
//! Day 8, checkpoint 3: add the builtin catalog here.
// pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>>;
// pub const BUILTIN_EXPRESSION_NAMES: &[&str];
//
//! Day 10, checkpoint 1: add one representative batch loop choice here. Select it only from checked physical `Nullability` metadata while keeping one primitive array representation.
//     /* fn output_nullability(&self, inputs: &[Nullability]) -> Nullability */
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
