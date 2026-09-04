use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, PhysicalType, Scalar,
    ScalarRefImpl, TypeMismatch,
};

/// Validate arity, physical types, and row counts before evaluation reads any rows.
pub fn validate_expression_inputs(
    inputs: &[ColumnViewImpl<'_>],
    expected_types: &[PhysicalType],
) -> anyhow::Result<usize> {
    if inputs.len() != expected_types.len() {
        anyhow::bail!(
            "input arity mismatch: expected {}, got {}",
            expected_types.len(),
            inputs.len()
        );
    }

    for (input_index, (input, expected)) in inputs.iter().zip(expected_types).enumerate() {
        if input.physical_type() != *expected {
            anyhow::bail!(
                "input {input_index} type mismatch: expected {expected:?}, got {:?}",
                input.physical_type()
            );
        }
    }

    let len = inputs.first().map_or(0, ColumnViewImpl::len);
    for (input_index, input) in inputs.iter().enumerate().skip(1) {
        if input.len() != len {
            anyhow::bail!(
                "input {input_index} length mismatch: expected {len}, got {}",
                input.len()
            );
        }
    }
    Ok(len)
}

fn evaluate_typed_unary<'a, I, O, F>(input: ColumnView<'a, I>, function: &F) -> ArrayImpl
where
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

fn evaluate_typed_binary<'a, L, R, O, F>(
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

fn evaluate_typed_ternary<'a, A, B, C, O, F>(
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

/// Lift one infallible scalar function through the typed-`get` unary fallback.
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
    Ok(evaluate_typed_unary(
        ColumnView::<I>::try_from(input)?,
        &function,
    ))
}

/// Lift one infallible scalar function through the typed-`get` binary fallback.
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
    Ok(evaluate_typed_binary(
        ColumnView::<L>::try_from(left)?,
        ColumnView::<R>::try_from(right)?,
        &function,
    ))
}

/// Lift one infallible scalar function through the typed-`get` ternary fallback.
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
