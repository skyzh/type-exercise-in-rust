#![forbid(unsafe_code)]
#![allow(dead_code, unused_imports)] // Chapter 8 connects these physical factories to the registry.

mod boolean;
mod numeric;
mod string;
pub(crate) use string::{
    build_string_comparison_expression, build_string_concat_expression,
    build_string_contains_expression,
};

pub use boolean::*;
pub use numeric::*;
pub use type_exercise_checkpoint_07_core::*;
