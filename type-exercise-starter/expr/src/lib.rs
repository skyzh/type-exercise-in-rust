#![forbid(unsafe_code)]

pub use type_exercise_starter_core::*;

// This crate is the concrete-expression facade. The sibling core package owns storage, views, and
// shared traversal; this crate chooses concrete scalar operations.

// Checkpoint 3: enable the first numeric instantiations.
// mod numeric;
// pub use numeric::*;

// Checkpoint 8: enable Boolean and String scalar semantics together with the physical catalog.
// mod boolean;
// pub use boolean::*;

// mod string;
// pub use string::*;

// mod catalog;
// pub use catalog::*;

// Checkpoint 9: enable logical binding and coercion after physical selection is complete.
// mod binder;
// pub use binder::*;
