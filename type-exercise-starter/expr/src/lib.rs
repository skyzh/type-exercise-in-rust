#![forbid(unsafe_code)]

pub use type_exercise_starter_core::*;

// This crate is the concrete-expression facade. The sibling core package owns storage, views,
// evaluator families and representation dispatch.

// Chapter 3: enable the first concrete numeric instantiations.
// mod numeric;
// pub use numeric::*;

// Chapter 8: enable the private Boolean module and add its physical factories.
// mod boolean;
// pub use boolean::*;

// Chapter 8: enable variable-width scalar operations and their physical factories. Chapter 4
// introduced only the core writer publication boundary.
// mod string;
// pub use string::*;

// Chapter 8: after expanding numeric.rs and enabling Boolean/String factories, enable logical
// binding as the sole builtin catalog.
// mod binder;
// pub use binder::*;
