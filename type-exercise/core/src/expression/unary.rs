//! Unary Array/Constant specializations and their permanent Indexed fallback.

use crate::{
    ArrayImpl, ColumnView, ColumnViewImpl, ColumnViewKind, Scalar, ScalarRefImpl, TypeMismatch,
};

use super::fallback::{
    ArrayColumn, ConstantColumn, evaluate_nullable_unary_loop, evaluate_typed_unary,
    evaluate_unary_loop,
};
use super::validate_expression_inputs;

/// Lift one infallible scalar function through the shared typed-`get` fallback.
pub fn evaluate_unary<I, O, F>(input: ColumnViewImpl<'_>, function: F) -> anyhow::Result<ArrayImpl>
where
    I: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(I) -> O,
    for<'a> I: Scalar<RefType<'a> = I>,
    for<'a> &'a I::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> I::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(std::slice::from_ref(&input), &[I::PHYSICAL_TYPE])?;
    let input = ColumnView::<I>::try_from(input)?;
    Ok(evaluate_typed_unary(input, &function))
}

/// Specialize Array and Constant unary inputs while leaving Indexed on the
/// shared typed-`get` fallback.
pub fn auto_vectorize_unary<I, O, F>(
    input: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    I: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(I) -> O,
    for<'a> I: Scalar<RefType<'a> = I>,
    for<'a> &'a I::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> I::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(std::slice::from_ref(&input), &[I::PHYSICAL_TYPE])?;
    let input = ColumnView::<I>::try_from(input)?;
    Ok(match input.kind {
        ColumnViewKind::Array(array) => evaluate_unary_loop(ArrayColumn::<I> { array }, &function),
        ColumnViewKind::Constant { value, len } => {
            evaluate_unary_loop(ConstantColumn::<I> { value, len }, &function)
        }
        kind @ ColumnViewKind::Indexed { .. } => {
            evaluate_typed_unary(ColumnView { kind }, &function)
        }
    })
}

/// Lift one nullable-aware scalar function over a unary column.
pub fn evaluate_nullable_unary<I, O, F>(
    input: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    I: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(Option<I>) -> anyhow::Result<Option<O>>,
    for<'a> I: Scalar<RefType<'a> = I>,
    for<'a> &'a I::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> I::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(std::slice::from_ref(&input), &[I::PHYSICAL_TYPE])?;
    let input = ColumnView::<I>::try_from(input)?;
    evaluate_nullable_unary_loop(input, function)
}
