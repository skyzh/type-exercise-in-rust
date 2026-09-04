#![forbid(unsafe_code)]

pub use type_exercise_starter_core::*;

// This crate is the concrete-expression facade. The sibling core package owns storage, views, and
// shared traversal; this crate chooses concrete scalar operations.

// Checkpoint 3: enable the first numeric instantiations.
// mod numeric;
// pub use numeric::*;

// Day 8, checkpoint 1: enable the private module, implement scalar NOT/AND/OR, and expose the
// public Boolean expression surface.
// mod boolean;
// pub use boolean::*;
// Checkpoint 2: add public metadata and verify array-backed and invalid batches.

// Day 10: enable variable-width scalar operations after the core writer evaluator exists.
// mod string;
// pub use string::*;

// Day 11: enable logical binding and the builtin catalog.
// mod binder;
// pub use binder::*;
