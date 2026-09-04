#![forbid(unsafe_code)]

pub use type_exercise_starter_core::*;

// This crate is the concrete-expression facade. The sibling core package owns storage, views,
// evaluator families and representation dispatch.

// Chapter 3: enable the first concrete numeric instantiations.
// mod numeric;
// pub use numeric::*;

// Chapter 6: enable the private module, implement scalar NOT/AND/OR, and expose the
// public Boolean expression surface.
// mod boolean;
// pub use boolean::*;
// Add public metadata and verify array-backed and invalid batches in the same chapter.

// Chapter 4: enable variable-width scalar operations after the core writer evaluator exists.
// mod string;
// pub use string::*;

// Chapter 8: enable logical binding and the builtin catalog.
// mod binder;
// pub use binder::*;
