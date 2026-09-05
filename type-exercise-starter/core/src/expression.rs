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
//! Shape and raw-buffer specializations, semantic exceptions, runtime expression erasure,
//! registries, List evaluation, and asynchronous evaluation belong to later checkpoints.
