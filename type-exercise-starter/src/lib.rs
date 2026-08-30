#![forbid(unsafe_code)]

pub use type_exercise_starter_core::*;

// This crate is the concrete-expression facade. The nested core package owns storage, views,
// evaluator families, erasure, and the generic registry from Day 1 onward.

// Day 4: enable the first scalar arithmetic implementation.
// mod arithmetic;
// pub use arithmetic::*;

// Day 5: enable scalar numeric comparison; promotion selection is exported by the core package.
// mod comparison;
// pub use comparison::*;

// Day 8: enable strict NOT and nullable-aware AND/OR scalar operations.
// mod boolean;
// pub use boolean::*;

// Day 10: enable variable-width scalar operations after the core writer evaluator exists.
// mod string;
// pub use string::*;

// Day 11: enable logical binding and the builtin catalog.
// mod binder;
// pub use binder::*;

#[cfg(test)]
mod tests;
