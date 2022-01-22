//! Utilities to vectorize scalar functions

use std::marker::PhantomData;

use anyhow::{anyhow, Result};

use super::Expression;
use crate::array::{Array, ArrayBuilder, ArrayImpl};
use crate::scalar::Scalar;
use crate::TypeMismatch;

/// A trait over all binary scalar functions, which takes `I1` and `I2` as input parameter, and
/// outputs array of type `O`.
pub trait BinaryExprFunc<I1: Array, I2: Array, O: Array> {
    fn eval<'a>(&self, i1: I1::RefItem<'a>, i2: I2::RefItem<'a>) -> O::OwnedItem;
}

/// Represents a binary expression which takes `I1` and `I2` as input parameter, and outputs array
/// of type `O`.
///
/// [`BinaryExpression`] automatically vectorizes the scalar function to a vectorized one, while
/// erasing the concreate array type. Therefore, users simply call
/// `BinaryExpression::eval(ArrayImpl, ArrayImpl)`, while developers only need to provide
/// implementation for functions like `cmp_le(i32, i32)`.
///
/// [`BinaryExpression`] also erases lifetime from each [`BinaryExprFunc`], so that we can pass
/// `&ArrayImpl` into the `eval_batch` function instead of specifying a lifetime.
pub struct BinaryExpression<I1: Array, I2: Array, O: Array, F> {
    expr: F,
    _phantom: PhantomData<(I1, I2, O)>,
}

/// Implement [`BinaryExpression`] for any given scalar function `F`.
///
/// Note that as we cannot add `From<&'a ArrayImpl>` bound on [`Array`], so we have to specify them
/// here.
impl<I1: Array, I2: Array, O: Array, F> BinaryExpression<I1, I2, O, F>
where
    for<'a> &'a I1: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a I2: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    F: BinaryExprFunc<I1, I2, O>,
{
    /// Create a binary expression from existing function
    ///
    /// Previously (maybe nightly toolchain 2021-12-15), this function is not possible to be
    /// compiled due to some lifetime diagnose bug in the Rust compiler.
    pub fn new(expr: F) -> Self {
        Self {
            expr,
            _phantom: PhantomData,
        }
    }

    /// Evaluate the expression with the given array.
    pub fn eval_batch(&self, i1: &ArrayImpl, i2: &ArrayImpl) -> Result<ArrayImpl> {
        let i1a: &I1 = i1.try_into()?;
        let i2a: &I2 = i2.try_into()?;
        assert_eq!(i1.len(), i2.len(), "array length mismatch");
        let mut builder: O::Builder = O::Builder::with_capacity(i1.len());
        for (i1, i2) in i1a.iter().zip(i2a.iter()) {
            match (i1, i2) {
                (Some(i1), Some(i2)) => builder.push(Some(self.expr.eval(i1, i2).as_scalar_ref())),
                _ => builder.push(None),
            }
        }
        Ok(builder.finish().into())
    }
}

impl<I1: Array, I2: Array, O: Array, F> Expression for BinaryExpression<I1, I2, O, F>
where
    for<'a> &'a I1: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a I2: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    F: BinaryExprFunc<I1, I2, O>,
{
    fn eval_expr(&self, data: &[&ArrayImpl]) -> Result<ArrayImpl> {
        if data.len() != 2 {
            return Err(anyhow!("Expect two inputs for BinaryExpression"));
        }
        self.eval_batch(data[0], data[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::{BoolArray, I32Array, I64Array, StringArray};
    use crate::expr::cmp::*;
    use crate::expr::string::*;

    /// Test if an array has the same content as a vector
    fn check_array_eq<'a, A: Array>(array: &'a A, vec: &[Option<A::RefItem<'a>>])
    where
        A::RefItem<'a>: PartialEq,
    {
        for (a, b) in array.iter().zip(vec.iter()) {
            assert_eq!(&a, b);
        }
    }

    #[test]
    fn test_cmp_le() {
        // Compare two `i32` array. Cast them to `i64` before comparing.
        let expr =
            BinaryExpression::<I32Array, I32Array, BoolArray, _>::new(ExprCmpLe::<_, _, I64Array>(
                PhantomData,
            ));
        let result = expr
            .eval_batch(
                &I32Array::from_slice(&[Some(0), Some(1), None]).into(),
                &I32Array::from_slice(&[Some(1), Some(0), None]).into(),
            )
            .unwrap();
        check_array_eq::<BoolArray>(
            (&result).try_into().unwrap(),
            &[Some(true), Some(false), None],
        );
    }

    #[test]
    fn test_cmp_ge_str() {
        let expr = BinaryExpression::<StringArray, StringArray, BoolArray, _>::new(ExprCmpGe::<
            _,
            _,
            StringArray,
        >(
            PhantomData
        ));
        let result = expr
            .eval_batch(
                &StringArray::from_slice(&[Some("0"), Some("1"), None]).into(),
                &StringArray::from_slice(&[Some("1"), Some("0"), None]).into(),
            )
            .unwrap();
        check_array_eq::<BoolArray>(
            (&result).try_into().unwrap(),
            &[Some(false), Some(true), None],
        );
    }

    #[test]
    fn test_str_contains() {
        let expr = BinaryExpression::<StringArray, StringArray, BoolArray, _>::new(ExprStrContains);
        let result = expr
            .eval_batch(
                &StringArray::from_slice(&[Some("000"), Some("111"), None]).into(),
                &StringArray::from_slice(&[Some("0"), Some("0"), None]).into(),
            )
            .unwrap();
        check_array_eq::<BoolArray>(
            (&result).try_into().unwrap(),
            &[Some(true), Some(false), None],
        );
    }
}
