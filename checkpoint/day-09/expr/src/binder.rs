//! Learner-owned erased-expression catalog and later binding checkpoints.
//!
//! Day 11: add the runtime registry and bound expression here.
// pub enum BindError { /* name, arity, and unsupported-signature failures */ }
// pub struct BoundExpression { /* logical metadata plus one selected expression */ }
// impl BoundExpression {
//     /* pub fn new(expression: Box<dyn Expression>, input_types, output_type) -> Result<Self, BindError> */
//     /* pub fn input_types(&self) -> &[DataType] */
//     /* pub fn output_type(&self) -> DataType */
//     /* pub fn physical_name(&self) -> &str */
//     /* pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> */
// }
// pub struct FunctionRegistry { /* registered factories */ }
// impl FunctionRegistry {
//     /* pub fn default() -> Self */
//     /* pub fn with_builtins() -> Self */
//     pub fn register(/* name and slice factory */);
//     pub fn register_unary(/* name and factory */);
//     pub fn register_binary(/* name and factory */);
//     pub fn register_ternary(/* name and factory */);
//     pub fn bind(&self, name: &str, inputs: &[DataType]) -> Result<BoundExpression, BindError>;
//     /* pub fn bind_binary(&self, name: &str, left: DataType, right: DataType) -> Result<BoundExpression, BindError> */
// }
//
//! Day 11: add arithmetic, comparison, and string binding helpers here.
// fn bind_arithmetic(/* operator and input types */) -> Result<BoundExpression, BindError>;
// fn bind_comparison(/* operator and input types */) -> Result<BoundExpression, BindError>;
//
//! Day 11: bind the Day 8 three-valued Boolean expressions here.
// fn bind_boolean(/* name, operator, and inputs */) -> Result<BoundExpression, BindError>;
//
//! Day 11: forward a preselected primitive loop here.
// impl BoundExpression {
//     pub fn evaluate_with_loop(
//         &self,
//         inputs: &[ColumnViewImpl<'_>],
//     ) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)>;
// }
//
//! Day 13: require registered factories to be Send + Sync + 'static here.
// impl FunctionRegistry { /* strengthen register, register_unary, register_binary, register_ternary */ }
//
//! Day 14: forward the already-bound expression through the async boundary here.
// impl AsyncExpression for BoundExpression {
//     fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
// }

use crate::{Expression, I32Add, PrimitiveBinaryExpression};

/// The fixed-width physical functions available at the Day 9 erased boundary.
pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &["i32_add"];

pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
    match name {
        "i32_add" => Some(Box::new(PrimitiveBinaryExpression::new("i32_add", I32Add))),
        _ => None,
    }
}
