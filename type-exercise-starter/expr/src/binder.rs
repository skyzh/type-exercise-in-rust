//! Learner-owned logical binding checkpoints.
//!
//! Chapter 8 binds logical names and types to one already-selected physical expression.
//! Checkpoint 1 adds `BindError`, `BoundExpression`, and a slice-arity `FunctionRegistry`.
//! Checkpoint 2 registers arithmetic, comparison, Boolean, and string factories with lossless
//! numeric promotion. The core package remains independent of this facade catalog.
//!
//! Chapter 10: strengthen the stored factory trait object and every registration method's factory
//! input to `Send + Sync + 'static`. The other Chapter 10 checks preserve completed Chapter 9 core
//! boundaries and require no new helper or core implementation.
//! Chapter 10 forwards a bound expression through the core async adapter.
