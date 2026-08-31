//! Learner-owned scalar arithmetic operations in the facade crate.
//!
//! Day 4 introduces `I32Add`: implement one scalar call and delegate the batch to the core
//! evaluator.
//!
//! Day 5 selects standard `Add`/`Sub`/`Mul`, explicit `Wrapping<T>` for signed overflow,
//! checked division, infallible `TryFrom` promotions, and ordered comparison once per batch.
//!
//! Day 6 adds only scalar functions such as:
//!
//! ```ignore
//! fn neg_number<O: Numeric>(value: O) -> O { /* one scalar operation */ }
//! fn clamp_number<O: Numeric>(value: O, lower: O, upper: O) -> anyhow::Result<O> {
//!     /* one scalar operation */
//! }
//! ```
//!
//! Generated typed adapters select physical types and call the shared core unary/binary/ternary
//! evaluators. This facade file must not contain a `for row in 0..` batch loop.
//!
//! Day 7 reuses the same scalar callbacks in the core's selected dense and general paths.
