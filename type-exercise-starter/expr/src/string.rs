//! Learner-owned variable-width scalar operations.
//!
//! Chapter 4 supplies only borrowed string logic. A callback consumes the core `Writer`, appends one
//! result directly into its buffers, and returns `WriterUsed`. It does not allocate with
//! `format!` and it owns no batch loop.
//! String comparison and containment selection also stay in this facade module once binding is
//! introduced; `numeric.rs` owns numeric operations only.
