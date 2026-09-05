//! Checkpoint 3: add the shared typed-`get` evaluation fallback here.
//!
//! Validate arity, physical types, and equal row counts before reading a row. Then implement the
//! three reusable public evaluators:
//!
//! ```rust,ignore
//! pub fn validate_expression_inputs(
//!     inputs: &[ColumnViewImpl<'_>],
//!     expected_types: &[PhysicalType],
//! ) -> anyhow::Result<usize>;
//!
//! pub fn evaluate_unary<I, O, F>(
//!     input: ColumnViewImpl<'_>,
//!     function: F,
//! ) -> anyhow::Result<ArrayImpl>;
//!
//! pub fn evaluate_binary<'a, L, R, O, F>(
//!     left: ColumnViewImpl<'a>,
//!     right: ColumnViewImpl<'a>,
//!     function: F,
//! ) -> anyhow::Result<ArrayImpl>;
//!
//! pub fn evaluate_ternary<A, B, C, O, F>(
//!     first: ColumnViewImpl<'_>,
//!     second: ColumnViewImpl<'_>,
//!     third: ColumnViewImpl<'_>,
//!     function: F,
//! ) -> anyhow::Result<ArrayImpl>;
//! ```
//!
//! Convert each erased view to `ColumnView<S>` once, call typed `get(row)` inside the shared loop,
//! skip the scalar function when any input is null, and build a fresh `O::ArrayType`. Do not match
//! on Array, Constant, or Indexed representations here.
//!
//! Checkpoint 4 adds `evaluate_writer_binary`. Validate two String inputs, convert them to typed
//! borrowed views once, and give each non-null pair a fresh consumed `Writer`; publish a null row
//! directly when either input is null. The callback must return `WriterUsed`, so it cannot skip or
//! repeat publication.
//!
//! Checkpoint 5 adds public `auto_vectorize_unary`, `auto_vectorize_binary`, and
//! `auto_vectorize_ternary` adapters. Specialize unary Array/Constant, the binary Array/Constant
//! cross-product, and ternary Array/Array/Array; send Indexed and every other ternary combination
//! through the existing typed-`get` fallback with identical validation and null behavior.
//!
//! Checkpoint 6 adds three public binary boundaries in this core module:
//!
//! - `auto_vectorize_primitive_i32` may combine raw values and validity for non-Indexed Int32
//!   Array/Constant shapes, while Indexed inputs keep the typed fallback;
//! - `try_evaluate_binary` validates first, skips null rows, and reports the first scalar error
//!   with row and function context; and
//! - `evaluate_nullable_binary` calls a fallible callback with both `Option` values on every row,
//!   enabling SQL three-valued Boolean logic.
//!
//! Checkpoint 7 adds a public `BatchKernel` function-pointer alias, a fixed-arity
//! `BatchExpression<const N: usize>`, and a dyn-compatible `Expression` trait. The fixed-arity
//! value owns its name and physical input/output contract. Validate every input before calling
//! its whole-batch kernel, then validate the returned array's physical type and row count.
//! Erasing the value behind `Box<dyn Expression>` must preserve the same metadata and evaluation.
//!
//! Checkpoint 8 adds `try_auto_vectorize_ternary`, the fallible counterpart of the existing
//! ternary adapter. Validate before any callback, skip null rows, stop at the first scalar error,
//! and return a fresh owned array. The physical `clamp` factory needs this core-owned traversal.
//!
//! ```rust,ignore
//! pub fn try_auto_vectorize_ternary<A, B, C, O, F, E>(
//!     first: ColumnViewImpl<'_>,
//!     second: ColumnViewImpl<'_>,
//!     third: ColumnViewImpl<'_>,
//!     function_name: &str,
//!     function: F,
//! ) -> anyhow::Result<ArrayImpl>
//! where
//!     F: Fn(A, B, C) -> Result<O, E>,
//!     E: std::fmt::Display;
//! ```
//!
//! Keep raw representation support private. Checkpoint 10 adds one-level List values and a
//! whole-batch asynchronous boundary over this existing synchronous expression contract.
//!
//! Before adding the four APIs below, strengthen the existing declaration to
//! `pub trait Expression: Send + Sync`. The borrowing future is `Send`, so both the referenced
//! expression and the erased child in `Box<dyn Expression>` must be safe to share across threads.
//!
//! ```rust,ignore
//! pub type BatchFuture<'a> =
//!     Pin<Box<dyn Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a>>;
//!
//! pub fn evaluate_static<'a, E>(
//!     expression: &'a E,
//!     inputs: &'a [ColumnViewImpl<'a>],
//! ) -> impl Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a
//! where
//!     E: Expression + ?Sized;
//!
//! pub trait AsyncExpression: Send + Sync {
//!     fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
//! }
//!
//! pub struct AsyncExpressionAdapter { /* owns one Box<dyn Expression> */ }
//! impl AsyncExpressionAdapter {
//!     pub fn new(expression: Box<dyn Expression>) -> Self;
//! }
//! ```
