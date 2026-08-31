//! Learner-owned logical binding checkpoints.
//!
//! Day 11 binds logical names and types to one already-selected physical expression.
//! Checkpoint 1 adds `BindError`, `BoundExpression`, and a slice-arity `FunctionRegistry`.
//! Checkpoint 2 registers arithmetic, comparison, Boolean, and string factories with lossless
//! numeric promotion. The core package remains independent of this facade catalog.
//!
//! Day 13 strengthens registered factories to `Send + Sync + 'static` and preserves borrow
//! shortening through `BoundExpression`.
//! Day 14 forwards a bound expression through the core async adapter.
