//! Permanent typed-`get` fallback loops shared by every vectorization tier.

use std::fmt::Display;

use crate::{Array, ArrayBuilder, ArrayImpl, ColumnView, Scalar};

pub(super) trait ColumnAccessor<'a, S: Scalar> {
    fn len(&self) -> usize;
    fn get(&self, row: usize) -> Option<S::RefType<'a>>;
}

pub(super) struct ArrayColumn<'a, S: Scalar> {
    pub(super) array: &'a S::ArrayType,
}

impl<'a, S: Scalar> ColumnAccessor<'a, S> for ArrayColumn<'a, S> {
    fn len(&self) -> usize {
        self.array.len()
    }

    fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        let array: &'a S::ArrayType = self.array;
        array.get(row)
    }
}

pub(super) struct ConstantColumn<'a, S: Scalar> {
    pub(super) value: Option<S::RefType<'a>>,
    pub(super) len: usize,
}

impl<'a, S: Scalar> ColumnAccessor<'a, S> for ConstantColumn<'a, S> {
    fn len(&self) -> usize {
        self.len
    }

    fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        assert!(row < self.len, "column view row out of bounds");
        self.value
    }
}

impl<'a, S: Scalar> ColumnAccessor<'a, S> for ColumnView<'a, S> {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        ColumnView::get(self, row)
    }
}

pub(super) fn evaluate_unary_loop<'a, C, I, O, F>(input: C, function: &F) -> ArrayImpl
where
    C: ColumnAccessor<'a, I>,
    I: Scalar,
    O: Scalar,
    F: Fn(I::RefType<'a>) -> O,
{
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(input.len());
    for row in 0..input.len() {
        let value = input.get(row).map(function);
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    output.finish().into()
}

pub(super) fn evaluate_binary_loop<'a, C1, C2, L, R, O, F>(
    left: C1,
    right: C2,
    function: &F,
) -> ArrayImpl
where
    C1: ColumnAccessor<'a, L>,
    C2: ColumnAccessor<'a, R>,
    L: Scalar,
    R: Scalar,
    O: Scalar,
    F: Fn(L::RefType<'a>, R::RefType<'a>) -> O,
{
    debug_assert_eq!(left.len(), right.len());
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = left
            .get(row)
            .zip(right.get(row))
            .map(|(left, right)| function(left, right));
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    output.finish().into()
}

pub(super) fn evaluate_typed_unary<'a, I, O, F>(input: ColumnView<'a, I>, function: &F) -> ArrayImpl
where
    I: Scalar,
    O: Scalar,
    F: Fn(I::RefType<'a>) -> O,
{
    evaluate_unary_loop(input, function)
}

pub(super) fn evaluate_typed_binary<'a, L, R, O, F>(
    left: ColumnView<'a, L>,
    right: ColumnView<'a, R>,
    function: &F,
) -> ArrayImpl
where
    L: Scalar,
    R: Scalar,
    O: Scalar,
    F: Fn(L::RefType<'a>, R::RefType<'a>) -> O,
{
    evaluate_binary_loop(left, right, function)
}

pub(super) fn evaluate_ternary_loop<'a, C1, C2, C3, A, B, C, O, F>(
    first: C1,
    second: C2,
    third: C3,
    function: &F,
) -> ArrayImpl
where
    C1: ColumnAccessor<'a, A>,
    C2: ColumnAccessor<'a, B>,
    C3: ColumnAccessor<'a, C>,
    A: Scalar,
    B: Scalar,
    C: Scalar,
    O: Scalar,
    F: Fn(A::RefType<'a>, B::RefType<'a>, C::RefType<'a>) -> O,
{
    debug_assert_eq!(first.len(), second.len());
    debug_assert_eq!(first.len(), third.len());
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(first.len());
    for row in 0..first.len() {
        let value = first
            .get(row)
            .zip(second.get(row))
            .zip(third.get(row))
            .map(|((first, second), third)| function(first, second, third));
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    output.finish().into()
}

pub(super) fn evaluate_typed_ternary<'a, A, B, C, O, F>(
    first: ColumnView<'a, A>,
    second: ColumnView<'a, B>,
    third: ColumnView<'a, C>,
    function: &F,
) -> ArrayImpl
where
    A: Scalar,
    B: Scalar,
    C: Scalar,
    O: Scalar,
    F: Fn(A::RefType<'a>, B::RefType<'a>, C::RefType<'a>) -> O,
{
    evaluate_ternary_loop(first, second, third, function)
}

pub(super) fn try_evaluate_ternary_loop<'a, C1, C2, C3, A, B, C, O, F, E>(
    first: C1,
    second: C2,
    third: C3,
    function_name: &str,
    function: &F,
) -> anyhow::Result<ArrayImpl>
where
    C1: ColumnAccessor<'a, A>,
    C2: ColumnAccessor<'a, B>,
    C3: ColumnAccessor<'a, C>,
    A: Scalar,
    B: Scalar,
    C: Scalar,
    O: Scalar,
    F: Fn(A::RefType<'a>, B::RefType<'a>, C::RefType<'a>) -> Result<O, E>,
    E: Display,
{
    debug_assert_eq!(first.len(), second.len());
    debug_assert_eq!(first.len(), third.len());
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(first.len());
    for row in 0..first.len() {
        let value = match (first.get(row), second.get(row), third.get(row)) {
            (Some(first), Some(second), Some(third)) => {
                Some(function(first, second, third).map_err(|error| {
                    anyhow::anyhow!("function `{function_name}` failed at row {row}: {error}")
                })?)
            }
            _ => None,
        };
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

pub(super) fn evaluate_nullable_unary_loop<'a, I, O, F>(
    input: ColumnView<'a, I>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    I: Scalar,
    O: Scalar,
    F: Fn(Option<I::RefType<'a>>) -> anyhow::Result<Option<O>>,
{
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(input.len());
    for row in 0..input.len() {
        let value = function(input.get(row))?;
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

pub(super) fn evaluate_nullable_binary_loop<'a, L, R, O, F>(
    left: ColumnView<'a, L>,
    right: ColumnView<'a, R>,
    mut function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar,
    R: Scalar,
    O: Scalar,
    F: FnMut(Option<L::RefType<'a>>, Option<R::RefType<'a>>) -> anyhow::Result<Option<O>>,
{
    debug_assert_eq!(left.len(), right.len());
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = function(left.get(row), right.get(row))?;
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}
