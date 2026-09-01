use crate::{Expression, I32Add, PrimitiveBinaryExpression};

pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &["i32_add", "string_concat"];

pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
    match name {
        "i32_add" => Some(Box::new(PrimitiveBinaryExpression::new("i32_add", I32Add))),
        "string_concat" => Some(Box::new(crate::string::build_string_concat_expression())),
        _ => None,
    }
}
