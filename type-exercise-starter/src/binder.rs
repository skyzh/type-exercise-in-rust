//! Learner-owned binding checkpoints.
//!
//! Day 9, checkpoint 1: add the runtime registry and bound expression here.
// pub enum BindError { /* name, arity, and unsupported-signature failures */ }
// pub struct BoundExpression { /* logical metadata plus one selected expression */ }
// pub struct FunctionRegistry { /* registered factories */ }
// impl FunctionRegistry {
//     pub fn register(/* name and factory */);
//     pub fn register_unary(/* name and factory */);
//     pub fn register_binary(/* name and factory */);
//     pub fn register_ternary(/* name and factory */);
//     pub fn bind(&self, name: &str, inputs: &[DataType]) -> Result<BoundExpression, BindError>;
// }
//
//! Day 9, checkpoint 2: add arithmetic, comparison, and string binding helpers here.
// fn bind_arithmetic(/* operator and input types */) -> Result<BoundExpression, BindError>;
// fn bind_comparison(/* operator and input types */) -> Result<BoundExpression, BindError>;
//
//! Day 10, checkpoint 2: forward a preselected primitive loop here.
// impl BoundExpression {
//     pub fn evaluate_with_loop(/* inputs and loop choice */) -> Result<ArrayImpl, ExpressionError>;
// }
//
//! Day 12, checkpoint 3: require registered factories to be Send + Sync + 'static here.
// impl FunctionRegistry { /* strengthen register, register_unary, register_binary, register_ternary */ }
//
//! Day 13, checkpoint 3: forward the already-bound expression through the async boundary here.
// impl AsyncExpression for BoundExpression {
//     fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
// }
