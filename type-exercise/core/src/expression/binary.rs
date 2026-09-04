//! Binary shape specializations and their permanent Indexed fallback.

use std::fmt::Display;

use crate::{
    ArrayImpl, ColumnView, ColumnViewImpl, ColumnViewKind, Scalar, ScalarRefImpl, TypeMismatch,
};

use super::fallback::{
    ArrayColumn, ConstantColumn, evaluate_binary_loop, evaluate_nullable_binary_loop,
    evaluate_typed_binary,
};
use super::validate_expression_inputs;

/// Lift one infallible scalar function through the shared typed-`get` fallback.
pub fn evaluate_binary<'a, L, R, O, F>(
    left: ColumnViewImpl<'a>,
    right: ColumnViewImpl<'a>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar,
    R: Scalar,
    O: Scalar,
    F: Fn(L::RefType<'a>, R::RefType<'a>) -> O,
    L::ArrayType: 'a,
    R::ArrayType: 'a,
    &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[L::PHYSICAL_TYPE, R::PHYSICAL_TYPE],
    )?;
    let left = ColumnView::<L>::try_from(left)?;
    let right = ColumnView::<R>::try_from(right)?;
    Ok(evaluate_typed_binary(left, right, &function))
}

/// Lift one infallible scalar function over two nullable columns.
///
/// Array/Array, Array/Constant, Constant/Array, and Constant/Constant receive
/// concrete loops. Any Indexed input stays on the shared typed-`get` fallback.
pub fn auto_vectorize_binary<L, R, O, F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar + Copy,
    R: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(L, R) -> O,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[L::PHYSICAL_TYPE, R::PHYSICAL_TYPE],
    )?;
    let left = ColumnView::<L>::try_from(left)?;
    let right = ColumnView::<R>::try_from(right)?;
    Ok(match (left.kind, right.kind) {
        (ColumnViewKind::Array(left), ColumnViewKind::Array(right)) => evaluate_binary_loop(
            ArrayColumn::<L> { array: left },
            ArrayColumn::<R> { array: right },
            &function,
        ),
        (ColumnViewKind::Array(left), ColumnViewKind::Constant { value, len }) => {
            evaluate_binary_loop(
                ArrayColumn::<L> { array: left },
                ConstantColumn::<R> { value, len },
                &function,
            )
        }
        (ColumnViewKind::Constant { value, len }, ColumnViewKind::Array(right)) => {
            evaluate_binary_loop(
                ConstantColumn::<L> { value, len },
                ArrayColumn::<R> { array: right },
                &function,
            )
        }
        (
            ColumnViewKind::Constant { value: left, len },
            ColumnViewKind::Constant { value: right, .. },
        ) => evaluate_binary_loop(
            ConstantColumn::<L> { value: left, len },
            ConstantColumn::<R> { value: right, len },
            &function,
        ),
        (left_kind, right_kind) => evaluate_typed_binary(
            ColumnView { kind: left_kind },
            ColumnView { kind: right_kind },
            &function,
        ),
    })
}

/// Lift one fallible scalar function over two nullable columns.
pub fn try_evaluate_binary<L, R, O, F, E>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function_name: &str,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar + Copy,
    R: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(L, R) -> Result<O, E>,
    E: Display,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    let mut row = 0;
    evaluate_nullable_binary::<L, R, O, _>(left, right, |left, right| {
        let result = left
            .zip(right)
            .map(|(left, right)| function(left, right))
            .transpose()
            .map_err(|error| {
                if function_name.is_empty() {
                    anyhow::anyhow!("row {row}: {error}")
                } else {
                    anyhow::anyhow!("function `{function_name}` failed at row {row}: {error}")
                }
            });
        row += 1;
        result
    })
}

/// Lift one nullable-aware scalar function over two columns.
pub fn evaluate_nullable_binary<L, R, O, F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar + Copy,
    R: Scalar + Copy,
    O: Scalar + Copy,
    F: FnMut(Option<L>, Option<R>) -> anyhow::Result<Option<O>>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[L::PHYSICAL_TYPE, R::PHYSICAL_TYPE],
    )?;
    let left = ColumnView::<L>::try_from(left)?;
    let right = ColumnView::<R>::try_from(right)?;
    evaluate_nullable_binary_loop(left, right, function)
}
