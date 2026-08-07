use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, Scalar, ScalarRefImpl, TypeMismatch,
};

/// A checked failure at the typed-expression boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionError {
    TypeMismatch(TypeMismatch),
    InputLengthMismatch { left: usize, right: usize },
}

impl Display for ExpressionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeMismatch(error) => Display::fmt(error, formatter),
            Self::InputLengthMismatch { left, right } => {
                write!(
                    formatter,
                    "input length mismatch: left {left}, right {right}"
                )
            }
        }
    }
}

impl Error for ExpressionError {}

impl From<TypeMismatch> for ExpressionError {
    fn from(error: TypeMismatch) -> Self {
        Self::TypeMismatch(error)
    }
}

/// One typed binary scalar function that can be lifted over nullable columns.
pub trait BinaryScalarFunction {
    type Left: Scalar;
    type Right: Scalar;
    type Output: Scalar;

    fn evaluate<'a>(
        &self,
        left: <Self::Left as Scalar>::RefType<'a>,
        right: <Self::Right as Scalar>::RefType<'a>,
    ) -> Self::Output;
}

/// The Chapter 3 scalar function. Addition uses explicit wrapping semantics.
#[derive(Clone, Copy, Debug, Default)]
pub struct I32Add;

impl BinaryScalarFunction for I32Add {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate<'a>(&self, left: i32, right: i32) -> i32 {
        left.wrapping_add(right)
    }
}

/// Convert erased inputs once, then apply a typed scalar function row by row.
pub fn evaluate_binary<'a, F>(
    function: &F,
    left: ColumnViewImpl<'a>,
    right: ColumnViewImpl<'a>,
) -> Result<ArrayImpl, ExpressionError>
where
    F: BinaryScalarFunction,
    <F::Left as Scalar>::ArrayType: 'a,
    <F::Right as Scalar>::ArrayType: 'a,
    &'a <F::Left as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    &'a <F::Right as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    <F::Left as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    <F::Right as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    let left = ColumnView::<F::Left>::try_from(left)?;
    let right = ColumnView::<F::Right>::try_from(right)?;
    if left.len() != right.len() {
        return Err(ExpressionError::InputLengthMismatch {
            left: left.len(),
            right: right.len(),
        });
    }

    let mut output =
        <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => Some(function.evaluate(left, right)),
            _ => None,
        };
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}
