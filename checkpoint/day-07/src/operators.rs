#![allow(dead_code)]

use std::cmp::Ordering;
use std::num::Wrapping;
use std::ops::{Add, Mul, Neg, Sub};

use crate::{
    Array, ArrayBuilder, ArrayImpl, BinaryExpression, ColumnView, ColumnViewImpl, PhysicalType,
    Scalar, ScalarRefImpl, TypeMismatch,
};

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

/// One monomorphized evaluator for a complete input batch.
pub type BatchKernel<const N: usize> =
    for<'a> fn(&BatchExpression<N>, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;

/// A fixed-arity expression whose only callable operation is vectorized.
pub struct BatchExpression<const N: usize> {
    name: &'static str,
    input_types: [PhysicalType; N],
    output_type: PhysicalType,
    kernel: BatchKernel<N>,
}

impl<const N: usize> BatchExpression<N> {
    pub fn new(
        name: &'static str,
        input_types: [PhysicalType; N],
        output_type: PhysicalType,
        kernel: BatchKernel<N>,
    ) -> Self {
        Self {
            name,
            input_types,
            output_type,
            kernel,
        }
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        validate_expression_inputs(inputs, &self.input_types)?;
        (self.kernel)(self, inputs)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonOperator {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

trait Numeric:
    Scalar
    + Copy
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + std::ops::Div<Output = Self>
    + Neg<Output = Self>
{
    type Arithmetic: Add<Output = Self::Arithmetic>
        + Sub<Output = Self::Arithmetic>
        + Mul<Output = Self::Arithmetic>
        + Neg<Output = Self::Arithmetic>;

    fn into_arithmetic(self) -> Self::Arithmetic;
    fn from_arithmetic(value: Self::Arithmetic) -> Self;
}

impl Numeric for i16 {
    type Arithmetic = Wrapping<Self>;

    fn into_arithmetic(self) -> Self::Arithmetic {
        Wrapping(self)
    }

    fn from_arithmetic(value: Self::Arithmetic) -> Self {
        value.0
    }
}

impl Numeric for i32 {
    type Arithmetic = Wrapping<Self>;

    fn into_arithmetic(self) -> Self::Arithmetic {
        Wrapping(self)
    }

    fn from_arithmetic(value: Self::Arithmetic) -> Self {
        value.0
    }
}

impl Numeric for i64 {
    type Arithmetic = Wrapping<Self>;

    fn into_arithmetic(self) -> Self::Arithmetic {
        Wrapping(self)
    }

    fn from_arithmetic(value: Self::Arithmetic) -> Self {
        value.0
    }
}

impl Numeric for f32 {
    type Arithmetic = Self;

    fn into_arithmetic(self) -> Self::Arithmetic {
        self
    }

    fn from_arithmetic(value: Self::Arithmetic) -> Self {
        value
    }
}

impl Numeric for f64 {
    type Arithmetic = Self;

    fn into_arithmetic(self) -> Self::Arithmetic {
        self
    }

    fn from_arithmetic(value: Self::Arithmetic) -> Self {
        value
    }
}

trait CheckedDivide: Sized {
    fn checked_divide(self, rhs: Self) -> anyhow::Result<Self>;
}

impl CheckedDivide for i16 {
    fn checked_divide(self, rhs: Self) -> anyhow::Result<Self> {
        if rhs == 0 {
            anyhow::bail!("division by zero");
        }
        self.checked_div(rhs)
            .ok_or_else(|| anyhow::anyhow!("signed integer division overflow"))
    }
}

impl CheckedDivide for i32 {
    fn checked_divide(self, rhs: Self) -> anyhow::Result<Self> {
        if rhs == 0 {
            anyhow::bail!("division by zero");
        }
        self.checked_div(rhs)
            .ok_or_else(|| anyhow::anyhow!("signed integer division overflow"))
    }
}

impl CheckedDivide for i64 {
    fn checked_divide(self, rhs: Self) -> anyhow::Result<Self> {
        if rhs == 0 {
            anyhow::bail!("division by zero");
        }
        self.checked_div(rhs)
            .ok_or_else(|| anyhow::anyhow!("signed integer division overflow"))
    }
}

impl CheckedDivide for f32 {
    fn checked_divide(self, rhs: Self) -> anyhow::Result<Self> {
        if rhs == 0.0 {
            anyhow::bail!("division by zero")
        } else {
            Ok(self / rhs)
        }
    }
}

impl CheckedDivide for f64 {
    fn checked_divide(self, rhs: Self) -> anyhow::Result<Self> {
        if rhs == 0.0 {
            anyhow::bail!("division by zero")
        } else {
            Ok(self / rhs)
        }
    }
}

fn add_number<O: Numeric>(left: O, right: O) -> O {
    O::from_arithmetic(Add::add(left.into_arithmetic(), right.into_arithmetic()))
}

fn subtract_number<O: Numeric>(left: O, right: O) -> O {
    O::from_arithmetic(Sub::sub(left.into_arithmetic(), right.into_arithmetic()))
}

fn multiply_number<O: Numeric>(left: O, right: O) -> O {
    O::from_arithmetic(Mul::mul(left.into_arithmetic(), right.into_arithmetic()))
}

fn divide_number<O: CheckedDivide>(left: O, right: O) -> anyhow::Result<O> {
    left.checked_divide(right)
}

fn neg_number<O: Numeric>(value: O) -> O {
    O::from_arithmetic(Neg::neg(value.into_arithmetic()))
}

fn clamp_number<O: Numeric>(value: O, lower: O, upper: O) -> anyhow::Result<O> {
    if lower.partial_cmp(&upper) != Some(Ordering::Less)
        && lower.partial_cmp(&upper) != Some(Ordering::Equal)
    {
        anyhow::bail!("invalid clamp bounds");
    } else if value < lower {
        Ok(lower)
    } else if value > upper {
        Ok(upper)
    } else {
        Ok(value)
    }
}

fn lossless_try_from<T, U>(value: U) -> T
where
    T: TryFrom<U, Error = std::convert::Infallible>,
{
    T::try_from(value).unwrap_or_else(|never| match never {})
}

/// Auto-vectorize one scalar function over a nullable unary batch.
pub(crate) fn evaluate_unary<I, O, F>(
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
    evaluate_nullable_unary::<I, O, _>(input, |value| Ok(value.map(&function)))
}

/// Auto-vectorize one nullable unary scalar function.
pub(crate) fn evaluate_nullable_unary<I, O, F>(
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
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(input.len());
    for row in 0..input.len() {
        let value = function(input.get(row))?;
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

/// Auto-vectorize one infallible scalar function over two nullable batches.
pub(crate) fn auto_vectorize_binary<L, R, O, F>(
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
    evaluate_nullable_binary::<L, R, O, _>(left, right, |left, right| {
        Ok(left.zip(right).map(|(left, right)| function(left, right)))
    })
}

/// Auto-vectorize one fallible scalar function and attach row context.
pub(crate) fn try_evaluate_binary<L, R, O, F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function_name: &str,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar + Copy,
    R: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(L, R) -> anyhow::Result<O>,
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

/// Auto-vectorize one nullable binary scalar function.
pub(crate) fn evaluate_nullable_binary<L, R, O, F>(
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

/// Auto-vectorize one fallible ternary scalar function and attach row context.
pub(crate) fn try_evaluate_ternary<A, B, C, O, F>(
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
    F: Fn(A, B, C) -> anyhow::Result<O>,
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

type NumericBinaryBatchKernel = for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
type NumericComparisonBatchKernel =
    for<'a> fn(&NumericComparisonExpression, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
type NumericNegBatchKernel =
    for<'a> fn(&NumericNegExpression, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
type NumericClampBatchKernel =
    for<'a> fn(&NumericClampExpression, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
pub(crate) struct NumericComparisonExpression {
    name: &'static str,
    input_types: [PhysicalType; 2],
    operator: ComparisonOperator,
    kernel: NumericComparisonBatchKernel,
}

impl NumericComparisonExpression {
    pub(crate) fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        validate_expression_inputs(inputs, &self.input_types)?;
        (self.kernel)(self, inputs)
    }
}

pub(crate) struct NumericNegExpression {
    name: &'static str,
    input_types: [PhysicalType; 1],
    kernel: NumericNegBatchKernel,
}

impl NumericNegExpression {
    pub(crate) fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        validate_expression_inputs(inputs, &self.input_types)?;
        (self.kernel)(self, inputs)
    }
}

pub(crate) struct NumericClampExpression {
    name: &'static str,
    input_types: [PhysicalType; 3],
    output_type: PhysicalType,
    kernel: NumericClampBatchKernel,
}

impl NumericClampExpression {
    pub(crate) fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        validate_expression_inputs(inputs, &self.input_types)?;
        let output = (self.kernel)(self, inputs)?;
        if output.physical_type() != self.output_type {
            anyhow::bail!(
                "output type mismatch: expected {:?}, got {:?}",
                self.output_type,
                output.physical_type()
            );
        }
        Ok(output)
    }
}

fn evaluate_numeric_infallible<L, R, O, Operation>(
    inputs: &[ColumnViewImpl<'_>],
    operation: Operation,
) -> anyhow::Result<ArrayImpl>
where
    L: Numeric,
    R: Numeric,
    O: Numeric
        + TryFrom<L, Error = std::convert::Infallible>
        + TryFrom<R, Error = std::convert::Infallible>,
    Operation: Fn(O, O) -> O,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    auto_vectorize_binary::<L, R, O, _>(inputs[0].clone(), inputs[1].clone(), |left, right| {
        let left = lossless_try_from::<O, L>(left);
        let right = lossless_try_from::<O, R>(right);
        operation(left, right)
    })
}

fn evaluate_numeric_add<L, R, O>(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>
where
    L: Numeric,
    R: Numeric,
    O: Numeric
        + TryFrom<L, Error = std::convert::Infallible>
        + TryFrom<R, Error = std::convert::Infallible>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    evaluate_numeric_infallible::<L, R, O, _>(inputs, add_number::<O>)
}

fn evaluate_numeric_subtract<L, R, O>(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>
where
    L: Numeric,
    R: Numeric,
    O: Numeric
        + TryFrom<L, Error = std::convert::Infallible>
        + TryFrom<R, Error = std::convert::Infallible>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    evaluate_numeric_infallible::<L, R, O, _>(inputs, subtract_number::<O>)
}

fn evaluate_numeric_multiply<L, R, O>(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>
where
    L: Numeric,
    R: Numeric,
    O: Numeric
        + TryFrom<L, Error = std::convert::Infallible>
        + TryFrom<R, Error = std::convert::Infallible>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    evaluate_numeric_infallible::<L, R, O, _>(inputs, multiply_number::<O>)
}

fn evaluate_numeric_divide<L, R, O>(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>
where
    L: Numeric,
    R: Numeric,
    O: Numeric
        + CheckedDivide
        + TryFrom<L, Error = std::convert::Infallible>
        + TryFrom<R, Error = std::convert::Infallible>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    try_evaluate_binary::<L, R, O, _>(inputs[0].clone(), inputs[1].clone(), "", |left, right| {
        let left = lossless_try_from::<O, L>(left);
        let right = lossless_try_from::<O, R>(right);
        divide_number(left, right)
    })
}

fn numeric_binary_kernel<L, R, O>(operator: ArithmeticOperator) -> NumericBinaryBatchKernel
where
    L: Numeric,
    R: Numeric,
    O: Numeric
        + CheckedDivide
        + TryFrom<L, Error = std::convert::Infallible>
        + TryFrom<R, Error = std::convert::Infallible>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    match operator {
        ArithmeticOperator::Add => evaluate_numeric_add::<L, R, O>,
        ArithmeticOperator::Subtract => evaluate_numeric_subtract::<L, R, O>,
        ArithmeticOperator::Multiply => evaluate_numeric_multiply::<L, R, O>,
        ArithmeticOperator::Divide => evaluate_numeric_divide::<L, R, O>,
    }
}

fn evaluate_numeric_comparison<L, R, O>(
    expression: &NumericComparisonExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl>
where
    L: Numeric,
    R: Numeric,
    O: Numeric
        + TryFrom<L, Error = std::convert::Infallible>
        + TryFrom<R, Error = std::convert::Infallible>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    auto_vectorize_binary::<L, R, bool, _>(inputs[0].clone(), inputs[1].clone(), |left, right| {
        let left = lossless_try_from::<O, L>(left);
        let right = lossless_try_from::<O, R>(right);
        match expression.operator {
            ComparisonOperator::Less => left < right,
            ComparisonOperator::LessOrEqual => left <= right,
            ComparisonOperator::Greater => left > right,
            ComparisonOperator::GreaterOrEqual => left >= right,
            ComparisonOperator::Equal => left == right,
            ComparisonOperator::NotEqual => left != right,
        }
    })
}

struct NumericKernels {
    binary: NumericBinaryBatchKernel,
    comparison: NumericComparisonBatchKernel,
}

fn numeric_kernels(
    operator: ArithmeticOperator,
    left: &PhysicalType,
    right: &PhysicalType,
    output: &PhysicalType,
) -> NumericKernels {
    match (left, right, output) {
        (PhysicalType::Int16, PhysicalType::Int16, PhysicalType::Int16) => NumericKernels {
            binary: numeric_binary_kernel::<i16, i16, i16>(operator),
            comparison: evaluate_numeric_comparison::<i16, i16, i16>,
        },
        (PhysicalType::Int16, PhysicalType::Int32, PhysicalType::Int32) => NumericKernels {
            binary: numeric_binary_kernel::<i16, i32, i32>(operator),
            comparison: evaluate_numeric_comparison::<i16, i32, i32>,
        },
        (PhysicalType::Int32, PhysicalType::Int16, PhysicalType::Int32) => NumericKernels {
            binary: numeric_binary_kernel::<i32, i16, i32>(operator),
            comparison: evaluate_numeric_comparison::<i32, i16, i32>,
        },
        (PhysicalType::Int16, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i16, i64, i64>(operator),
            comparison: evaluate_numeric_comparison::<i16, i64, i64>,
        },
        (PhysicalType::Int64, PhysicalType::Int16, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i64, i16, i64>(operator),
            comparison: evaluate_numeric_comparison::<i64, i16, i64>,
        },
        (PhysicalType::Int16, PhysicalType::Float32, PhysicalType::Float32) => NumericKernels {
            binary: numeric_binary_kernel::<i16, f32, f32>(operator),
            comparison: evaluate_numeric_comparison::<i16, f32, f32>,
        },
        (PhysicalType::Float32, PhysicalType::Int16, PhysicalType::Float32) => NumericKernels {
            binary: numeric_binary_kernel::<f32, i16, f32>(operator),
            comparison: evaluate_numeric_comparison::<f32, i16, f32>,
        },
        (PhysicalType::Int16, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<i16, f64, f64>(operator),
            comparison: evaluate_numeric_comparison::<i16, f64, f64>,
        },
        (PhysicalType::Float64, PhysicalType::Int16, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f64, i16, f64>(operator),
            comparison: evaluate_numeric_comparison::<f64, i16, f64>,
        },
        (PhysicalType::Int32, PhysicalType::Int32, PhysicalType::Int32) => NumericKernels {
            binary: numeric_binary_kernel::<i32, i32, i32>(operator),
            comparison: evaluate_numeric_comparison::<i32, i32, i32>,
        },
        (PhysicalType::Int32, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i32, i64, i64>(operator),
            comparison: evaluate_numeric_comparison::<i32, i64, i64>,
        },
        (PhysicalType::Int64, PhysicalType::Int32, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i64, i32, i64>(operator),
            comparison: evaluate_numeric_comparison::<i64, i32, i64>,
        },
        (PhysicalType::Int32, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<i32, f64, f64>(operator),
            comparison: evaluate_numeric_comparison::<i32, f64, f64>,
        },
        (PhysicalType::Int32, PhysicalType::Float32, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<i32, f32, f64>(operator),
            comparison: evaluate_numeric_comparison::<i32, f32, f64>,
        },
        (PhysicalType::Float32, PhysicalType::Int32, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f32, i32, f64>(operator),
            comparison: evaluate_numeric_comparison::<f32, i32, f64>,
        },
        (PhysicalType::Float64, PhysicalType::Int32, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f64, i32, f64>(operator),
            comparison: evaluate_numeric_comparison::<f64, i32, f64>,
        },
        (PhysicalType::Int64, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i64, i64, i64>(operator),
            comparison: evaluate_numeric_comparison::<i64, i64, i64>,
        },
        (PhysicalType::Float32, PhysicalType::Float32, PhysicalType::Float32) => NumericKernels {
            binary: numeric_binary_kernel::<f32, f32, f32>(operator),
            comparison: evaluate_numeric_comparison::<f32, f32, f32>,
        },
        (PhysicalType::Float32, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f32, f64, f64>(operator),
            comparison: evaluate_numeric_comparison::<f32, f64, f64>,
        },
        (PhysicalType::Float64, PhysicalType::Float32, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f64, f32, f64>(operator),
            comparison: evaluate_numeric_comparison::<f64, f32, f64>,
        },
        (PhysicalType::Float64, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f64, f64, f64>(operator),
            comparison: evaluate_numeric_comparison::<f64, f64, f64>,
        },
        _ => unreachable!("validated numeric promotion tuple"),
    }
}

fn evaluate_numeric_neg<O>(
    _expression: &NumericNegExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl>
where
    O: Numeric,
    for<'a> O: Scalar<RefType<'a> = O>,
    for<'a> &'a O::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> O::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    evaluate_unary::<O, O, _>(inputs[0].clone(), neg_number::<O>)
}

fn numeric_neg_kernel(input: &PhysicalType) -> NumericNegBatchKernel {
    match input {
        PhysicalType::Int16 => evaluate_numeric_neg::<i16>,
        PhysicalType::Int32 => evaluate_numeric_neg::<i32>,
        PhysicalType::Int64 => evaluate_numeric_neg::<i64>,
        PhysicalType::Float32 => evaluate_numeric_neg::<f32>,
        PhysicalType::Float64 => evaluate_numeric_neg::<f64>,
        _ => unreachable!("numeric negation input"),
    }
}

fn evaluate_numeric_clamp<A, B, C, O>(
    expression: &NumericClampExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl>
where
    A: Numeric,
    B: Numeric,
    C: Numeric,
    O: Numeric
        + TryFrom<A, Error = std::convert::Infallible>
        + TryFrom<B, Error = std::convert::Infallible>
        + TryFrom<C, Error = std::convert::Infallible>,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> C: Scalar<RefType<'a> = C>,
    for<'a> O: Scalar<RefType<'a> = O>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a C::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> C::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    try_evaluate_ternary::<A, B, C, O, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        inputs[2].clone(),
        expression.name,
        |value, lower, upper| {
            let value = lossless_try_from::<O, A>(value);
            let lower = lossless_try_from::<O, B>(lower);
            let upper = lossless_try_from::<O, C>(upper);
            clamp_number(value, lower, upper)
        },
    )
}

fn numeric_clamp_after_int16_pair<A, B>(
    third: &PhysicalType,
    output: &PhysicalType,
) -> NumericClampBatchKernel
where
    A: Numeric,
    B: Numeric,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    i16:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
    i32:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
    i64:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
    f32:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
    f64:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
{
    match (third, output) {
        (PhysicalType::Int16, PhysicalType::Int16) => evaluate_numeric_clamp::<A, B, i16, i16>,
        (PhysicalType::Int32, PhysicalType::Int32) => evaluate_numeric_clamp::<A, B, i32, i32>,
        (PhysicalType::Int64, PhysicalType::Int64) => evaluate_numeric_clamp::<A, B, i64, i64>,
        (PhysicalType::Float32, PhysicalType::Float32) => evaluate_numeric_clamp::<A, B, f32, f32>,
        (PhysicalType::Float64, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, f64, f64>,
        _ => unreachable!("validated numeric clamp tuple after Int16 pair"),
    }
}

fn numeric_clamp_after_int32_pair<A, B>(
    third: &PhysicalType,
    output: &PhysicalType,
) -> NumericClampBatchKernel
where
    A: Numeric,
    B: Numeric,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    i32:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
    i64:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
    f64:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
{
    match (third, output) {
        (PhysicalType::Int16, PhysicalType::Int32) => evaluate_numeric_clamp::<A, B, i16, i32>,
        (PhysicalType::Int32, PhysicalType::Int32) => evaluate_numeric_clamp::<A, B, i32, i32>,
        (PhysicalType::Int64, PhysicalType::Int64) => evaluate_numeric_clamp::<A, B, i64, i64>,
        (PhysicalType::Float32, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, f32, f64>,
        (PhysicalType::Float64, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, f64, f64>,
        _ => unreachable!("validated numeric clamp tuple after Int32 pair"),
    }
}

fn numeric_clamp_after_int64_pair<A, B>(
    third: &PhysicalType,
    output: &PhysicalType,
) -> NumericClampBatchKernel
where
    A: Numeric,
    B: Numeric,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    i64:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
{
    match (third, output) {
        (PhysicalType::Int16, PhysicalType::Int64) => evaluate_numeric_clamp::<A, B, i16, i64>,
        (PhysicalType::Int32, PhysicalType::Int64) => evaluate_numeric_clamp::<A, B, i32, i64>,
        (PhysicalType::Int64, PhysicalType::Int64) => evaluate_numeric_clamp::<A, B, i64, i64>,
        _ => unreachable!("validated numeric clamp tuple after Int64 pair"),
    }
}

fn numeric_clamp_after_float32_pair<A, B>(
    third: &PhysicalType,
    output: &PhysicalType,
) -> NumericClampBatchKernel
where
    A: Numeric,
    B: Numeric,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    f32:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
    f64:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
{
    match (third, output) {
        (PhysicalType::Int16, PhysicalType::Float32) => evaluate_numeric_clamp::<A, B, i16, f32>,
        (PhysicalType::Int32, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, i32, f64>,
        (PhysicalType::Float32, PhysicalType::Float32) => evaluate_numeric_clamp::<A, B, f32, f32>,
        (PhysicalType::Float64, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, f64, f64>,
        _ => unreachable!("validated numeric clamp tuple after Float32 pair"),
    }
}

fn numeric_clamp_after_float64_pair<A, B>(
    third: &PhysicalType,
    output: &PhysicalType,
) -> NumericClampBatchKernel
where
    A: Numeric,
    B: Numeric,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    f64:
        TryFrom<A, Error = std::convert::Infallible> + TryFrom<B, Error = std::convert::Infallible>,
{
    match (third, output) {
        (PhysicalType::Int16, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, i16, f64>,
        (PhysicalType::Int32, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, i32, f64>,
        (PhysicalType::Float32, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, f32, f64>,
        (PhysicalType::Float64, PhysicalType::Float64) => evaluate_numeric_clamp::<A, B, f64, f64>,
        _ => unreachable!("validated numeric clamp tuple after Float64 pair"),
    }
}

fn numeric_clamp_kernel(
    inputs: &[PhysicalType; 3],
    output: &PhysicalType,
) -> NumericClampBatchKernel {
    match (&inputs[0], &inputs[1]) {
        (PhysicalType::Int16, PhysicalType::Int16) => {
            numeric_clamp_after_int16_pair::<i16, i16>(&inputs[2], output)
        }
        (PhysicalType::Int16, PhysicalType::Int32) => {
            numeric_clamp_after_int32_pair::<i16, i32>(&inputs[2], output)
        }
        (PhysicalType::Int32, PhysicalType::Int16) => {
            numeric_clamp_after_int32_pair::<i32, i16>(&inputs[2], output)
        }
        (PhysicalType::Int32, PhysicalType::Int32) => {
            numeric_clamp_after_int32_pair::<i32, i32>(&inputs[2], output)
        }
        (PhysicalType::Int16, PhysicalType::Int64) => {
            numeric_clamp_after_int64_pair::<i16, i64>(&inputs[2], output)
        }
        (PhysicalType::Int64, PhysicalType::Int16) => {
            numeric_clamp_after_int64_pair::<i64, i16>(&inputs[2], output)
        }
        (PhysicalType::Int32, PhysicalType::Int64) => {
            numeric_clamp_after_int64_pair::<i32, i64>(&inputs[2], output)
        }
        (PhysicalType::Int64, PhysicalType::Int32) => {
            numeric_clamp_after_int64_pair::<i64, i32>(&inputs[2], output)
        }
        (PhysicalType::Int64, PhysicalType::Int64) => {
            numeric_clamp_after_int64_pair::<i64, i64>(&inputs[2], output)
        }
        (PhysicalType::Int16, PhysicalType::Float32) => {
            numeric_clamp_after_float32_pair::<i16, f32>(&inputs[2], output)
        }
        (PhysicalType::Float32, PhysicalType::Int16) => {
            numeric_clamp_after_float32_pair::<f32, i16>(&inputs[2], output)
        }
        (PhysicalType::Float32, PhysicalType::Float32) => {
            numeric_clamp_after_float32_pair::<f32, f32>(&inputs[2], output)
        }
        (PhysicalType::Int16, PhysicalType::Float64) => {
            numeric_clamp_after_float64_pair::<i16, f64>(&inputs[2], output)
        }
        (PhysicalType::Float64, PhysicalType::Int16) => {
            numeric_clamp_after_float64_pair::<f64, i16>(&inputs[2], output)
        }
        (PhysicalType::Int32, PhysicalType::Float32) => {
            numeric_clamp_after_float64_pair::<i32, f32>(&inputs[2], output)
        }
        (PhysicalType::Float32, PhysicalType::Int32) => {
            numeric_clamp_after_float64_pair::<f32, i32>(&inputs[2], output)
        }
        (PhysicalType::Int32, PhysicalType::Float64) => {
            numeric_clamp_after_float64_pair::<i32, f64>(&inputs[2], output)
        }
        (PhysicalType::Float64, PhysicalType::Int32) => {
            numeric_clamp_after_float64_pair::<f64, i32>(&inputs[2], output)
        }
        (PhysicalType::Float32, PhysicalType::Float64) => {
            numeric_clamp_after_float64_pair::<f32, f64>(&inputs[2], output)
        }
        (PhysicalType::Float64, PhysicalType::Float32) => {
            numeric_clamp_after_float64_pair::<f64, f32>(&inputs[2], output)
        }
        (PhysicalType::Float64, PhysicalType::Float64) => {
            numeric_clamp_after_float64_pair::<f64, f64>(&inputs[2], output)
        }
        _ => unreachable!("validated numeric clamp input pair"),
    }
}

pub(crate) fn build_numeric_binary_expression(
    name: &'static str,
    operator: ArithmeticOperator,
    left: PhysicalType,
    right: PhysicalType,
    output: PhysicalType,
) -> BinaryExpression {
    let kernel = numeric_kernels(operator, &left, &right, &output).binary;
    BinaryExpression::new_with_scalar_rows(name, [left, right], output, kernel)
}

pub(crate) fn build_numeric_neg_expression(
    name: &'static str,
    input: PhysicalType,
) -> NumericNegExpression {
    let kernel = numeric_neg_kernel(&input);
    NumericNegExpression {
        name,
        input_types: [input],
        kernel,
    }
}

pub(crate) fn build_numeric_clamp_expression(
    name: &'static str,
    inputs: [PhysicalType; 3],
    output: PhysicalType,
) -> NumericClampExpression {
    let kernel = numeric_clamp_kernel(&inputs, &output);
    NumericClampExpression {
        name,
        input_types: inputs,
        output_type: output,
        kernel,
    }
}

pub(crate) fn build_numeric_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
    left: PhysicalType,
    right: PhysicalType,
    common: PhysicalType,
) -> NumericComparisonExpression {
    let kernel = numeric_kernels(ArithmeticOperator::Add, &left, &right, &common).comparison;
    NumericComparisonExpression {
        name,
        input_types: [left, right],
        operator,
        kernel,
    }
}
