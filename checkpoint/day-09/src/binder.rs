use crate::{Expression, I32Add, PrimitiveBinaryExpression};

/// The fixed-width physical functions available at the erased boundary.
pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &["i32_add"];

pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
    match name {
        "i32_add" => Some(Box::new(PrimitiveBinaryExpression::new("i32_add", I32Add))),
        _ => None,
    }
}
