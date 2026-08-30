//! Learner-owned variable-width scalar operations.
//!
//! Day 10 supplies only borrowed string logic. A callback consumes the core `Writer`, appends one
//! result directly into its buffers, and returns `WriterUsed`. It does not allocate with
//! `format!` and it owns no batch loop.
