//! Expression framework based on array

use std::marker::PhantomData;

use anyhow::Result;

use self::vectorize::BinaryExpression;
use crate::array::ArrayImpl;

mod cmp;
mod string;
mod vectorize;

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

    use crate::array::*;
    use crate::expr::cmp::*;
    use crate::expr::string::*;

    match f {
        CmpLe => Box::new(BinaryExpression::<I32Array, I32Array, BoolArray, _>::new(
            ExprCmpLe::<_, _, I32Array>(PhantomData),
        )),
        CmpGe => Box::new(BinaryExpression::<I32Array, I32Array, BoolArray, _>::new(
            ExprCmpGe::<_, _, I32Array>(PhantomData),
        )),
        CmpEq => Box::new(BinaryExpression::<I32Array, I32Array, BoolArray, _>::new(
            ExprCmpEq::<_, _, I32Array>(PhantomData),
        )),
        CmpNe => Box::new(BinaryExpression::<I32Array, I32Array, BoolArray, _>::new(
            ExprCmpNe::<_, _, I32Array>(PhantomData),
        )),
        StrContains => Box::new(
            BinaryExpression::<StringArray, StringArray, BoolArray, _>::new(ExprStrContains),
        ),
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
