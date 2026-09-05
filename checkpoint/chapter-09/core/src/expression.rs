use std::fmt::Display;

use bitvec::vec::BitVec;

use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, ColumnViewKind, I32Array,
    PhysicalType, Scalar, ScalarRefImpl, TypeMismatch,
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

trait ColumnAccessor<'a, S: Scalar> {
    fn len(&self) -> usize;
    fn get(&self, row: usize) -> Option<S::RefType<'a>>;
}

struct ArrayColumn<'a, S: Scalar> {
    array: &'a S::ArrayType,
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

struct ConstantColumn<'a, S: Scalar> {
    value: Option<S::RefType<'a>>,
    len: usize,
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
        ColumnView::len(self)
    }

    fn get(&self, row: usize) -> Option<S::RefType<'a>> {
        ColumnView::get(self, row)
    }
}

fn evaluate_unary_loop<'a, C, I, O, F>(input: C, function: &F) -> ArrayImpl
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

fn evaluate_binary_loop<'a, C1, C2, L, R, O, F>(left: C1, right: C2, function: &F) -> ArrayImpl
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

fn evaluate_ternary_loop<'a, C1, C2, C3, A, B, C, O, F>(
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

fn try_evaluate_ternary_loop<'a, C1, C2, C3, A, B, C, O, F, E>(
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

/// Specialize Array and Constant unary inputs while keeping Indexed on the typed fallback.
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

/// Specialize the complete Array/Constant binary cross-product.
///
/// Any Indexed input remains on the shared typed fallback.
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
        (left, right) => evaluate_typed_binary(
            ColumnView { kind: left },
            ColumnView { kind: right },
            &function,
        ),
    })
}

#[derive(Clone, Copy)]
enum RawI32Column<'a> {
    Array {
        values: &'a [i32],
        validity: &'a BitVec,
    },
    Constant {
        value: i32,
        valid: bool,
        len: usize,
    },
}

impl RawI32Column<'_> {
    fn len(self) -> usize {
        match self {
            Self::Array { values, .. } => values.len(),
            Self::Constant { len, .. } => len,
        }
    }

    fn value(self, row: usize) -> i32 {
        match self {
            Self::Array { values, .. } => values[row],
            Self::Constant { value, .. } => value,
        }
    }

    fn is_valid(self, row: usize) -> bool {
        match self {
            Self::Array { validity, .. } => validity[row],
            Self::Constant { valid, .. } => valid,
        }
    }
}

fn raw_i32_column(column: ColumnViewKind<'_, i32>) -> RawI32Column<'_> {
    match column {
        ColumnViewKind::Array(array) => RawI32Column::Array {
            values: array.values(),
            validity: array.validity(),
        },
        ColumnViewKind::Constant { value, len } => RawI32Column::Constant {
            value: value.unwrap_or_default(),
            valid: value.is_some(),
            len,
        },
        ColumnViewKind::Indexed { .. } => unreachable!("Indexed stays on the typed fallback"),
    }
}

/// Lift one strict, total, infallible Int32 function through raw values and validity.
///
/// Any Indexed input remains on the shared typed fallback.
pub fn auto_vectorize_primitive_i32<F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: Fn(i32, i32) -> i32,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[PhysicalType::Int32, PhysicalType::Int32],
    )?;
    let left = ColumnView::<i32>::try_from(left)?;
    let right = ColumnView::<i32>::try_from(right)?;
    if matches!(&left.kind, ColumnViewKind::Indexed { .. })
        || matches!(&right.kind, ColumnViewKind::Indexed { .. })
    {
        return Ok(evaluate_typed_binary(left, right, &function));
    }

    let left = raw_i32_column(left.kind);
    let right = raw_i32_column(right.kind);
    debug_assert_eq!(left.len(), right.len());
    let len = left.len();
    let values = (0..len)
        .map(|row| function(left.value(row), right.value(row)))
        .collect();
    let validity = (0..len)
        .map(|row| left.is_valid(row) && right.is_valid(row))
        .collect();
    Ok(I32Array::from_raw_parts(values, validity).into())
}

/// Lift one fallible strict scalar function over two nullable columns.
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
    mut function: F,
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
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = function(left.get(row), right.get(row))?;
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

/// Specialize the common Array/Array/Array ternary shape.
///
/// Every Constant or Indexed combination remains on the shared typed fallback.
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

/// Lift one fallible scalar function over three strict nullable inputs.
///
/// The common Array/Array/Array shape is specialized. Every other shape uses the same typed
/// fallback, preserving validation-before-callback, null skipping, and the first scalar error.
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

/// Lift one strict string scalar function through the transactional writer loop.
pub fn evaluate_writer_binary<F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: for<'a> Fn(&str, &str, crate::Writer<'a>) -> crate::WriterUsed<'a>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[PhysicalType::String, PhysicalType::String],
    )?;
    let left = ColumnView::<String>::try_from(left)?;
    let right = ColumnView::<String>::try_from(right)?;
    let mut output = <crate::StringArray as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => {
                function(left, right, output.writer()).into_builder();
            }
            _ => output.push_null(),
        }
    }
    Ok(output.finish().into())
}

/// One monomorphized evaluator for a complete input batch.
pub type BatchKernel = for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;

/// A fixed-arity physical expression with discoverable input and output types.
pub struct BatchExpression<const N: usize> {
    name: &'static str,
    input_types: [PhysicalType; N],
    output_type: PhysicalType,
    kernel: BatchKernel,
}

impl<const N: usize> BatchExpression<N> {
    pub fn new(
        name: &'static str,
        input_types: [PhysicalType; N],
        output_type: PhysicalType,
        kernel: BatchKernel,
    ) -> Self {
        Self {
            name,
            input_types,
            output_type,
            kernel,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn input_types(&self) -> &[PhysicalType; N] {
        &self.input_types
    }

    pub fn output_type(&self) -> PhysicalType {
        self.output_type.clone()
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        let input_len = validate_expression_inputs(inputs, &self.input_types)?;
        let output = (self.kernel)(inputs)?;
        if output.physical_type() != self.output_type {
            anyhow::bail!(
                "output type mismatch: expected {:?}, got {:?}",
                self.output_type,
                output.physical_type()
            );
        }
        if output.len() != input_len {
            anyhow::bail!(
                "output length mismatch: expected {input_len}, got {}",
                output.len()
            );
        }
        Ok(output)
    }
}

/// A runtime-erased expression whose complete-batch contract remains inspectable.
pub trait Expression {
    fn name(&self) -> &'static str;
    fn input_types(&self) -> &[PhysicalType];
    fn arity(&self) -> usize {
        self.input_types().len()
    }
    fn output_type(&self) -> PhysicalType;
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>;
}

impl<const N: usize> Expression for BatchExpression<N> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> PhysicalType {
        self.output_type.clone()
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate(inputs)
    }
}
