//! Ternary Array/Array/Array specialization and permanent typed fallback.

use std::fmt::Display;

use crate::{
    ArrayImpl, ColumnView, ColumnViewImpl, ColumnViewKind, Scalar, ScalarRefImpl, TypeMismatch,
};

use super::fallback::{
    ArrayColumn, evaluate_ternary_loop, evaluate_typed_ternary, try_evaluate_ternary_loop,
};
use super::validate_expression_inputs;

/// Lift one infallible scalar function through the shared typed-`get` fallback.
pub fn evaluate_ternary<A, B, C, O, F>(
    first: ColumnViewImpl<'_>,
    second: ColumnViewImpl<'_>,
    third: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    A: Scalar + Copy,
    B: Scalar + Copy,
    C: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(A, B, C) -> O,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> C: Scalar<RefType<'a> = C>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a C::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> C::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[first.clone(), second.clone(), third.clone()],
        &[A::PHYSICAL_TYPE, B::PHYSICAL_TYPE, C::PHYSICAL_TYPE],
    )?;
    Ok(evaluate_typed_ternary(
        ColumnView::<A>::try_from(first)?,
        ColumnView::<B>::try_from(second)?,
        ColumnView::<C>::try_from(third)?,
        &function,
    ))
}

/// Specialize the common Array/Array/Array shape while every Constant or
/// Indexed combination uses the permanent shared typed-`get` fallback.
pub fn auto_vectorize_ternary<A, B, C, O, F>(
    first: ColumnViewImpl<'_>,
    second: ColumnViewImpl<'_>,
    third: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    A: Scalar + Copy,
    B: Scalar + Copy,
    C: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(A, B, C) -> O,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> C: Scalar<RefType<'a> = C>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a C::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> C::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[first.clone(), second.clone(), third.clone()],
        &[A::PHYSICAL_TYPE, B::PHYSICAL_TYPE, C::PHYSICAL_TYPE],
    )?;
    let first = ColumnView::<A>::try_from(first)?;
    let second = ColumnView::<B>::try_from(second)?;
    let third = ColumnView::<C>::try_from(third)?;
    Ok(match (first.kind, second.kind, third.kind) {
        (
            ColumnViewKind::Array(first),
            ColumnViewKind::Array(second),
            ColumnViewKind::Array(third),
        ) => evaluate_ternary_loop(
            ArrayColumn::<A> { array: first },
            ArrayColumn::<B> { array: second },
            ArrayColumn::<C> { array: third },
            &function,
        ),
        (first, second, third) => evaluate_typed_ternary(
            ColumnView { kind: first },
            ColumnView { kind: second },
            ColumnView { kind: third },
            &function,
        ),
    })
}

/// Lift one fallible scalar function over three nullable columns while
/// specializing only the Array/Array/Array shape.
pub fn try_auto_vectorize_ternary<A, B, C, O, F, E>(
    first: ColumnViewImpl<'_>,
    second: ColumnViewImpl<'_>,
    third: ColumnViewImpl<'_>,
    function_name: &str,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    A: Scalar + Copy,
    B: Scalar + Copy,
    C: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(A, B, C) -> Result<O, E>,
    E: Display,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> C: Scalar<RefType<'a> = C>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a C::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> C::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[first.clone(), second.clone(), third.clone()],
        &[A::PHYSICAL_TYPE, B::PHYSICAL_TYPE, C::PHYSICAL_TYPE],
    )?;
    let first = ColumnView::<A>::try_from(first)?;
    let second = ColumnView::<B>::try_from(second)?;
    let third = ColumnView::<C>::try_from(third)?;
    match (first.kind, second.kind, third.kind) {
        (
            ColumnViewKind::Array(first),
            ColumnViewKind::Array(second),
            ColumnViewKind::Array(third),
        ) => try_evaluate_ternary_loop(
            ArrayColumn::<A> { array: first },
            ArrayColumn::<B> { array: second },
            ArrayColumn::<C> { array: third },
            function_name,
            &function,
        ),
        (first, second, third) => try_evaluate_ternary_loop(
            ColumnView { kind: first },
            ColumnView { kind: second },
            ColumnView { kind: third },
            function_name,
            &function,
        ),
    }
}

/// Lift one fallible scalar function through the permanent typed-`get` fallback.
pub fn try_evaluate_ternary<A, B, C, O, F, E>(
    first: ColumnViewImpl<'_>,
    second: ColumnViewImpl<'_>,
    third: ColumnViewImpl<'_>,
    function_name: &str,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    A: Scalar + Copy,
    B: Scalar + Copy,
    C: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(A, B, C) -> Result<O, E>,
    E: Display,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> C: Scalar<RefType<'a> = C>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a C::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> C::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[first.clone(), second.clone(), third.clone()],
        &[A::PHYSICAL_TYPE, B::PHYSICAL_TYPE, C::PHYSICAL_TYPE],
    )?;
    try_evaluate_ternary_loop(
        ColumnView::<A>::try_from(first)?,
        ColumnView::<B>::try_from(second)?,
        ColumnView::<C>::try_from(third)?,
        function_name,
        &function,
    )
}
