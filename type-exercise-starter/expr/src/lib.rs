#![forbid(unsafe_code)]

pub use type_exercise_starter_core::*;

// This crate is the concrete-expression facade. The sibling core package owns storage, views,
// evaluator families, erasure, and the generic registry from Day 1 onward.

// Day 4: enable the first scalar arithmetic implementation.
// mod numeric;
// pub use numeric::*;

// Day 8, checkpoint 1: enable the private module and implement scalar NOT/AND/OR.
// mod boolean;
// Checkpoint 2: expose the completed Boolean expression surface.
// pub use boolean::*;

// Day 10: enable variable-width scalar operations after the core writer evaluator exists.
// mod string;
// pub use string::*;

// Day 11: enable logical binding and the builtin catalog.
// mod binder;
// pub use binder::*;
