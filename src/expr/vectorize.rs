use std::marker::PhantomData;

use anyhow::{anyhow, Result};

use super::Expression;
use crate::array::{Array, ArrayBuilder, ArrayImpl};
use crate::scalar::Scalar;
use crate::TypeMismatch;

/// A trait over all scalar SQL functions.
///
/// It takes `A` and `B` as input parameter, and outputs scalar of type `O`.
pub trait BinaryExprFunc<A: Scalar, B: Scalar, O: Scalar> {
    /// Evaluate a binary function with two references to data.
    fn eval(&self, i1: A::RefType<'_>, i2: B::RefType<'_>) -> O;
}

/// Represents a binary expression which takes `I1` and `I2` as input parameter, and outputs scalar
/// of type `O`.
///
/// [`BinaryExpression`] automatically vectorizes the scalar function to a vectorized one, while
/// erasing the concreate array type. Therefore, users simply call
/// `BinaryExpression::eval(ArrayImpl, ArrayImpl)`, while developers only need to provide
/// implementation for functions like `cmp_le(i32, i32)`.
pub struct BinaryExpression<I1: Scalar, I2: Scalar, O: Scalar, F> {
    func: F,
    _phantom: PhantomData<(I1, I2, O)>,
}

/// Blanket implementation for all binary expression functions
impl<A: Scalar, B: Scalar, O: Scalar, F> BinaryExprFunc<A, B, O> for F
where
    F: Fn(A::RefType<'_>, B::RefType<'_>) -> O,
{
    fn eval(&self, i1: A::RefType<'_>, i2: B::RefType<'_>) -> O {
        self(i1, i2)
    }
}

/// Implement [`BinaryExpression`] for any given scalar function `F`.
///
/// Note that as we cannot add `From<&'a ArrayImpl>` bound on [`Array`], so we have to specify them
/// here.
impl<I1, I2, O, F> BinaryExpression<I1, I2, O, F>
where
    O: Scalar,
    I1: Scalar,
    I2: Scalar,
    for<'a> &'a I1::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a I2::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    F: BinaryExprFunc<I1, I2, O>,
{
    /// Create a binary expression from existing function
    ///
    /// Previously (maybe nightly toolchain 2021-12-15), this function is not possible to be
    /// compiled due to some lifetime diagnose bug in the Rust compiler.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: PhantomData,
        }
    }

    /// Evaluate the expression with the given array.
    pub fn eval_batch(&self, i1: &ArrayImpl, i2: &ArrayImpl) -> Result<ArrayImpl> {
        let i1a: &I1::ArrayType = i1.try_into()?;
        let i2a: &I2::ArrayType = i2.try_into()?;
        assert_eq!(i1.len(), i2.len(), "array length mismatch");
        let mut builder = <O::ArrayType as Array>::Builder::with_capacity(i1.len());
        for (i1, i2) in i1a.iter().zip(i2a.iter()) {
            match (i1, i2) {
                (Some(i1), Some(i2)) => builder.push(Some(self.func.eval(i1, i2).as_scalar_ref())),
                _ => builder.push(None),
            }
        }
        Ok(builder.finish().into())
    }
}

/// Blanket [`Expression`] implementation for [`BinaryExpression`]
impl<I1, I2, O, F> Expression for BinaryExpression<I1, I2, O, F>
where
    O: Scalar,
    I1: Scalar,
    I2: Scalar,
    for<'a> &'a I1::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a I2::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
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

    fn test_if_impl<A: Scalar, B: Scalar, O: Scalar, F: BinaryExprFunc<A, B, O>>(_: F) {}

    fn binary_str(_: &str, _: &str) -> String {
        todo!()
    }

    fn binary_generics<A: Scalar, B: Scalar, O: Scalar>(_: A::RefType<'_>, _: B::RefType<'_>) -> O {
        todo!()
    }

    #[test]
    fn test_simple_str_function() {
        // FIXME: test_if_impl(binary_str)
        //
        // Confusing compiler error with the above line:
        // ```plain
        // error[E0631]: type mismatch in function arguments
        //   --> archive/day6-hard/src/expr/vectorize.rs:99:22
        //    |
        // 89 |     fn binary_str(_: &str, _: &str) -> String {
        //    |     ----------------------------------------- found signature of `for<'r, 's> fn(&'r
        // str, &'s str) -> _` ...
        // 99 |         test_if_impl(binary_str)
        //    |         ------------ ^^^^^^^^^^ expected signature of `for<'r, 's> fn(<_ as
        // scalar::Scalar>::RefType<'r>, <_ as scalar::Scalar>::RefType<'s>) -> _`    |
        // |    |         required by a bound introduced by this call
        // ```

        test_if_impl::<String, String, String, _>(binary_str)
    }

    #[test]
    fn test_simple_generics_function() {
        test_if_impl::<i32, f32, i64, _>(binary_generics::<i32, f32, i64>)
    }

    use crate::array::{BoolArray, I32Array, StringArray};
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
        let expr = BinaryExpression::<i32, i32, bool, _>::new(cmp_le::<i32, i32, i64>);
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
        let expr =
            BinaryExpression::<String, String, bool, _>::new(cmp_ge::<String, String, String>);
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
        let expr = BinaryExpression::<String, String, bool, _>::new(str_contains);
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

    #[test]
    fn test_str_contains_lambda() {
        let expr =
            BinaryExpression::<String, String, bool, _>::new(|x1: &str, x2: &str| x1.contains(x2));
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
