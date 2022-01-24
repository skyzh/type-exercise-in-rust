// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Expression framework based on array

mod cmp;
mod string;
mod vectorize;

use anyhow::Result;

use crate::array::ArrayImpl;

/// A trait over all expressions -- unary, binary, etc.
pub trait Expression {
    /// Evaluate an expression with run-time number of [`ArrayImpl`]s.
    fn eval_expr(&self, data: &[&ArrayImpl]) -> Result<ArrayImpl>;
}

/// All supported expression functions
pub enum ExpressionFunc {
    CmpLe,
    CmpGe,
    CmpEq,
    CmpNe,
    StrContains,
}

/// Build expression with runtime information.
pub fn build_binary_expression(f: ExpressionFunc) -> Box<dyn Expression> {
    use ExpressionFunc::*;

    use crate::expr::cmp::*;
    use crate::expr::string::*;
    use crate::expr::vectorize::*;

    match f {
        CmpLe => Box::new(BinaryExpression::<i32, i32, bool, _>::new(
            cmp_le::<i32, i32, i64>,
        )),
        CmpGe => Box::new(BinaryExpression::<i32, i32, bool, _>::new(
            cmp_ge::<i32, i32, i64>,
        )),
        CmpEq => Box::new(BinaryExpression::<i32, i32, bool, _>::new(
            cmp_eq::<i32, i32, i64>,
        )),
        CmpNe => Box::new(BinaryExpression::<i32, i32, bool, _>::new(
            cmp_ne::<i32, i32, i64>,
        )),
        StrContains => Box::new(BinaryExpression::<String, String, bool, _>::new(
            str_contains,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::{Array, StringArray};
    use crate::scalar::ScalarRefImpl;

    #[test]
    fn test_build_str_contains() {
        let expr = build_binary_expression(ExpressionFunc::StrContains);

        for _ in 0..10 {
            let result = expr
                .eval_expr(&[
                    &StringArray::from_slice(&[Some("000"), Some("111"), None]).into(),
                    &StringArray::from_slice(&[Some("0"), Some("0"), None]).into(),
                ])
                .unwrap();
            assert_eq!(result.get(0).unwrap(), ScalarRefImpl::Bool(true));
            assert_eq!(result.get(1).unwrap(), ScalarRefImpl::Bool(false));
            assert!(result.get(2).is_none());
        }
    }
}
