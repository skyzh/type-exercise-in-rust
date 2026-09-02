//! Learner-owned logical binding checkpoints.
//!
//! Day 11 binds logical names and types to one already-selected physical expression.
//! Checkpoint 1 adds `BindError`, `BoundExpression`, and a slice-arity `FunctionRegistry`.
//! Checkpoint 2 registers arithmetic, comparison, Boolean, and string factories with lossless
//! numeric promotion. The core package remains independent of this facade catalog.
//!
//! Day 13: strengthen the stored factory trait object and every registration method's factory
//! input to `Send + Sync + 'static`. The other Chapter 13 checks preserve completed Day 12 core
//! boundaries and require no new helper or core implementation.
//! Day 14 forwards a bound expression through the core async adapter.
