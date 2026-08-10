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
// pub trait Expression { /* metadata and batch evaluation */ }
// pub struct BinaryExpression<F> { /* typed function plus runtime metadata */ }
//
//! Day 8, checkpoint 3: add the builtin catalog here.
// macro_rules! define_builtin_expressions { /* names and constructors */ }
// pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>>;
// pub const BUILTIN_EXPRESSION_NAMES: &[&str];
//
//! Day 10, checkpoint 1: add one representative batch loop choice here.
// pub enum PrimitiveLoop { /* supported dense loops plus general fallback */ }
// impl<F> PrimitiveBinaryExpression<F> {
//     pub fn evaluate_with_loop(&self, inputs: &[ColumnViewImpl<'_>], loop_kind: PrimitiveLoop)
//         -> Result<ArrayImpl, ExpressionError>;
// }
//
//! Day 12, checkpoint 2: strengthen the existing erased boundary here.
// pub trait Expression: Any + Send + Sync { /* checked Any recovery and evaluation */ }
//
//! Day 13, checkpoint 1: add one static batch future here.
// pub fn evaluate_static<'a, E>(expression: &'a E, inputs: &'a [ColumnViewImpl<'a>])
//     -> impl Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a;
//
//! Day 13, checkpoint 2: add the object-safe async adapter here.
// pub type BatchFuture<'a> = Pin<Box<dyn Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a>>;
// pub trait AsyncExpression: Send + Sync { /* evaluate_async */ }
// pub struct AsyncExpressionAdapter { /* one owned synchronous expression */ }
