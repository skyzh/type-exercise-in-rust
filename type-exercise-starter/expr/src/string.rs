//! Learner-owned variable-width scalar operations.
//!
//! Chapter 4 establishes the core `Writer` publication boundary with a supplied callback; it does
//! not enable this concrete facade.
//!
//! Chapter 8: implement borrowed concatenation, comparison, and containment semantics plus their
//! crate-private physical factories for the binder. Concatenation consumes the core `Writer`,
//! appends directly into its buffers, and returns `WriterUsed`; it does not allocate with `format!`
//! and owns no batch loop. `numeric.rs` owns numeric operations only.
