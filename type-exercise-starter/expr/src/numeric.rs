//! Learner-owned numeric instantiations.
//!
//! Chapter 3: define scalar semantics for mixed `i16 + i32 -> i32`, Int32 negation, and Int32
//! clamp. Each adapter calls the matching core `evaluate_unary`, `evaluate_binary`, or
//! `evaluate_ternary` function for the complete batch.
//!
//! This facade chooses concrete scalar types and operations. It must not contain a row loop or
//! inspect Array, Constant, or Indexed representations.
//!
//! Chapter 8: expand this module with physical operator metadata and crate-private factories that
//! select a complete numeric kernel and construct `BatchExpression<N>` for the binder.
