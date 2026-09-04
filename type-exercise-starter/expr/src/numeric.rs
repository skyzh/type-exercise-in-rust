//! Checkpoint 3: instantiate the first numeric operations here.
//!
//! Expose `add_i16_i32`, `negate_i32`, and `clamp_i32` with the signatures in the lesson. Each
//! adapter chooses concrete scalar types and supplies one scalar closure to core's corresponding
//! `evaluate_unary`, `evaluate_binary`, or `evaluate_ternary` function.
//!
//! Keep all row traversal in core. Do not inspect Array, Constant, or Indexed representations in
//! this facade module. Numeric operator metadata, promotion tables, fallible arithmetic, and
//! runtime factories belong to later checkpoints.
