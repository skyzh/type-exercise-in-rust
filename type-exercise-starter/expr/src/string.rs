//! Learner-owned variable-width scalar operations.
//!
//! Checkpoint 8 supplies borrowed string logic. Concatenation consumes the core `Writer`, appends
//! one result directly into its buffers, and returns `WriterUsed`; it does not allocate with
//! `format!` and it owns no batch loop. Comparisons and containment use the existing typed binary
//! evaluator. This module returns fixed-arity expressions but does not choose logical overloads.
