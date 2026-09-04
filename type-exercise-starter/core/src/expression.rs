//! Learner-owned evaluator templates.
//!
//! Chapter 3: add one input validator and the shared typed-get fallbacks:
//!
//! ```ignore
//! pub fn validate_expression_inputs(/* views and expected physical types */) -> anyhow::Result<usize>;
//! pub fn evaluate_unary<I, O, F>(/* one view and scalar function */) -> anyhow::Result<ArrayImpl>;
//! pub fn evaluate_binary<L, R, O, F>(/* two views and scalar function */) -> anyhow::Result<ArrayImpl>;
//! pub fn evaluate_ternary<A, B, C, O, F>(/* three views and scalar function */) -> anyhow::Result<ArrayImpl>;
//! ```
//!
//! Convert each erased view to `ColumnView<S>` once, call `.get(row)` inside the loop, propagate
//! nulls, and build `O::ArrayType`. Do not match Array/Constant/Indexed here.
//!
//! Later chapters add writer output, selective column-shape specialization, primitive Int32
//! lanes, one `BatchExpression<N>` shell, runtime erasure, and async adaptation. Leave those
//! declarations absent until their chapters introduce them.
