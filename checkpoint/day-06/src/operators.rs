#![allow(dead_code)]

use std::cmp::Ordering;
use std::num::Wrapping;
use std::ops::{Add, Mul, Neg, Sub};

use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, ExpressionError, PhysicalType,
    Scalar, ScalarError, ScalarRefImpl, TypeMismatch,
};

pub fn validate_expression_inputs(
    inputs: &[ColumnViewImpl<'_>],
    expected_types: &[PhysicalType],
) -> Result<usize, ExpressionError> {
    if inputs.len() != expected_types.len() {
        return Err(ExpressionError::InputArityMismatch {
            expected: expected_types.len(),
            actual: inputs.len(),
        });
    }
    for (input, expected) in inputs.iter().zip(expected_types) {
        if input.physical_type() != *expected {
            return Err(TypeMismatch {
                expected: expected.clone(),
                actual: input.physical_type(),
            }
            .into());
        }
    }
    let len = inputs.first().map_or(0, ColumnViewImpl::len);
    for (input_index, input) in inputs.iter().enumerate().skip(1) {
        if input.len() != len {
            return Err(ExpressionError::InputLengthMismatch {
                expected: len,
                actual: input.len(),
                input_index,
            });
        }
    }
    Ok(len)
}

/// One monomorphized evaluator for a complete input batch.
pub type BatchKernel<const N: usize> =
    for<'a> fn(&BatchExpression<N>, &[ColumnViewImpl<'a>]) -> Result<ArrayImpl, ExpressionError>;

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

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
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
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError>;
}

impl CheckedDivide for i16 {
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0 {
            return Err(ScalarError::DivisionByZero);
        }
        self.checked_div(rhs).ok_or(ScalarError::DivisionOverflow)
    }
}

impl CheckedDivide for i32 {
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0 {
            return Err(ScalarError::DivisionByZero);
        }
        self.checked_div(rhs).ok_or(ScalarError::DivisionOverflow)
    }
}

impl CheckedDivide for i64 {
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0 {
            return Err(ScalarError::DivisionByZero);
        }
        self.checked_div(rhs).ok_or(ScalarError::DivisionOverflow)
    }
}

impl CheckedDivide for f32 {
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0.0 {
            Err(ScalarError::DivisionByZero)
        } else {
            Ok(self / rhs)
        }
    }
}

impl CheckedDivide for f64 {
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0.0 {
            Err(ScalarError::DivisionByZero)
        } else {
            Ok(self / rhs)
        }
    }
}

fn lossless_try_from<T, U>(value: U) -> T
where
    T: TryFrom<U, Error = std::convert::Infallible>,
{
    T::try_from(value).unwrap_or_else(|never| match never {})
}

type NumericBinaryBatchKernel = for<'a> fn(
    &NumericBinaryExpression,
    &[ColumnViewImpl<'a>],
) -> Result<ArrayImpl, ExpressionError>;
type NumericComparisonBatchKernel = for<'a> fn(
    &NumericComparisonExpression,
    &[ColumnViewImpl<'a>],
) -> Result<ArrayImpl, ExpressionError>;
type NumericNegBatchKernel =
    for<'a> fn(&NumericNegExpression, &[ColumnViewImpl<'a>]) -> Result<ArrayImpl, ExpressionError>;
type NumericClampBatchKernel = for<'a> fn(
    &NumericClampExpression,
    &[ColumnViewImpl<'a>],
) -> Result<ArrayImpl, ExpressionError>;

pub(crate) struct NumericBinaryExpression {
    name: &'static str,
    input_types: [PhysicalType; 2],
    output_type: PhysicalType,
    operator: ArithmeticOperator,
    kernel: NumericBinaryBatchKernel,
}

impl NumericBinaryExpression {
    pub(crate) fn evaluate(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<ArrayImpl, ExpressionError> {
        (self.kernel)(self, inputs)
    }
}

pub(crate) struct NumericComparisonExpression {
    name: &'static str,
    input_types: [PhysicalType; 2],
    operator: ComparisonOperator,
    kernel: NumericComparisonBatchKernel,
}

impl NumericComparisonExpression {
    pub(crate) fn evaluate(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<ArrayImpl, ExpressionError> {
        (self.kernel)(self, inputs)
    }
}

pub(crate) struct NumericNegExpression {
    name: &'static str,
    input_types: [PhysicalType; 1],
    kernel: NumericNegBatchKernel,
}

impl NumericNegExpression {
    pub(crate) fn evaluate(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<ArrayImpl, ExpressionError> {
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
    pub(crate) fn evaluate(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<ArrayImpl, ExpressionError> {
        (self.kernel)(self, inputs)
    }
}

fn evaluate_numeric_binary<L, R, O>(
    expression: &NumericBinaryExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> Result<ArrayImpl, ExpressionError>
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
    let len = validate_expression_inputs(inputs, &expression.input_types)?;
    let left = ColumnView::<L>::try_from(inputs[0].clone())?;
    let right = ColumnView::<R>::try_from(inputs[1].clone())?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
    for row in 0..len {
        let value = match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => {
                let left = lossless_try_from::<O, L>(left);
                let right = lossless_try_from::<O, R>(right);
                let value = match expression.operator {
                    ArithmeticOperator::Add => Ok(O::from_arithmetic(Add::add(
                        left.into_arithmetic(),
                        right.into_arithmetic(),
                    ))),
                    ArithmeticOperator::Subtract => Ok(O::from_arithmetic(Sub::sub(
                        left.into_arithmetic(),
                        right.into_arithmetic(),
                    ))),
                    ArithmeticOperator::Multiply => Ok(O::from_arithmetic(Mul::mul(
                        left.into_arithmetic(),
                        right.into_arithmetic(),
                    ))),
                    ArithmeticOperator::Divide => left.checked_divide(right),
                }
                .map_err(|error| ExpressionError::ScalarEvaluation {
                    function: expression.name,
                    row,
                    error,
                })?;
                Some(value)
            }
            _ => None,
        };
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

fn evaluate_numeric_comparison<L, R, O>(
    expression: &NumericComparisonExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> Result<ArrayImpl, ExpressionError>
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
    let len = validate_expression_inputs(inputs, &expression.input_types)?;
    let left = ColumnView::<L>::try_from(inputs[0].clone())?;
    let right = ColumnView::<R>::try_from(inputs[1].clone())?;
    let mut output = <<bool as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
    for row in 0..len {
        let value = match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => {
                let left = lossless_try_from::<O, L>(left);
                let right = lossless_try_from::<O, R>(right);
                Some(match expression.operator {
                    ComparisonOperator::Less => left < right,
                    ComparisonOperator::LessOrEqual => left <= right,
                    ComparisonOperator::Greater => left > right,
                    ComparisonOperator::GreaterOrEqual => left >= right,
                    ComparisonOperator::Equal => left == right,
                    ComparisonOperator::NotEqual => left != right,
                })
            }
            _ => None,
        };
        output.push(value);
    }
    Ok(output.finish().into())
}

struct NumericKernels {
    binary: NumericBinaryBatchKernel,
    comparison: NumericComparisonBatchKernel,
}

fn numeric_kernels(
    left: &PhysicalType,
    right: &PhysicalType,
    output: &PhysicalType,
) -> NumericKernels {
    match (left, right, output) {
        (PhysicalType::Int16, PhysicalType::Int16, PhysicalType::Int16) => NumericKernels {
            binary: evaluate_numeric_binary::<i16, i16, i16>,
            comparison: evaluate_numeric_comparison::<i16, i16, i16>,
        },
        (PhysicalType::Int16, PhysicalType::Int32, PhysicalType::Int32) => NumericKernels {
            binary: evaluate_numeric_binary::<i16, i32, i32>,
            comparison: evaluate_numeric_comparison::<i16, i32, i32>,
        },
        (PhysicalType::Int32, PhysicalType::Int16, PhysicalType::Int32) => NumericKernels {
            binary: evaluate_numeric_binary::<i32, i16, i32>,
            comparison: evaluate_numeric_comparison::<i32, i16, i32>,
        },
        (PhysicalType::Int16, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: evaluate_numeric_binary::<i16, i64, i64>,
            comparison: evaluate_numeric_comparison::<i16, i64, i64>,
        },
        (PhysicalType::Int64, PhysicalType::Int16, PhysicalType::Int64) => NumericKernels {
            binary: evaluate_numeric_binary::<i64, i16, i64>,
            comparison: evaluate_numeric_comparison::<i64, i16, i64>,
        },
        (PhysicalType::Int16, PhysicalType::Float32, PhysicalType::Float32) => NumericKernels {
            binary: evaluate_numeric_binary::<i16, f32, f32>,
            comparison: evaluate_numeric_comparison::<i16, f32, f32>,
        },
        (PhysicalType::Float32, PhysicalType::Int16, PhysicalType::Float32) => NumericKernels {
            binary: evaluate_numeric_binary::<f32, i16, f32>,
            comparison: evaluate_numeric_comparison::<f32, i16, f32>,
        },
        (PhysicalType::Int16, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<i16, f64, f64>,
            comparison: evaluate_numeric_comparison::<i16, f64, f64>,
        },
        (PhysicalType::Float64, PhysicalType::Int16, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<f64, i16, f64>,
            comparison: evaluate_numeric_comparison::<f64, i16, f64>,
        },
        (PhysicalType::Int32, PhysicalType::Int32, PhysicalType::Int32) => NumericKernels {
            binary: evaluate_numeric_binary::<i32, i32, i32>,
            comparison: evaluate_numeric_comparison::<i32, i32, i32>,
        },
        (PhysicalType::Int32, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: evaluate_numeric_binary::<i32, i64, i64>,
            comparison: evaluate_numeric_comparison::<i32, i64, i64>,
        },
        (PhysicalType::Int64, PhysicalType::Int32, PhysicalType::Int64) => NumericKernels {
            binary: evaluate_numeric_binary::<i64, i32, i64>,
            comparison: evaluate_numeric_comparison::<i64, i32, i64>,
        },
        (PhysicalType::Int32, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<i32, f64, f64>,
            comparison: evaluate_numeric_comparison::<i32, f64, f64>,
        },
        (PhysicalType::Int32, PhysicalType::Float32, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<i32, f32, f64>,
            comparison: evaluate_numeric_comparison::<i32, f32, f64>,
        },
        (PhysicalType::Float32, PhysicalType::Int32, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<f32, i32, f64>,
            comparison: evaluate_numeric_comparison::<f32, i32, f64>,
        },
        (PhysicalType::Float64, PhysicalType::Int32, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<f64, i32, f64>,
            comparison: evaluate_numeric_comparison::<f64, i32, f64>,
        },
        (PhysicalType::Int64, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: evaluate_numeric_binary::<i64, i64, i64>,
            comparison: evaluate_numeric_comparison::<i64, i64, i64>,
        },
        (PhysicalType::Float32, PhysicalType::Float32, PhysicalType::Float32) => NumericKernels {
            binary: evaluate_numeric_binary::<f32, f32, f32>,
            comparison: evaluate_numeric_comparison::<f32, f32, f32>,
        },
        (PhysicalType::Float32, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<f32, f64, f64>,
            comparison: evaluate_numeric_comparison::<f32, f64, f64>,
        },
        (PhysicalType::Float64, PhysicalType::Float32, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<f64, f32, f64>,
            comparison: evaluate_numeric_comparison::<f64, f32, f64>,
        },
        (PhysicalType::Float64, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: evaluate_numeric_binary::<f64, f64, f64>,
            comparison: evaluate_numeric_comparison::<f64, f64, f64>,
        },
        _ => unreachable!("validated numeric promotion tuple"),
    }
}

fn evaluate_numeric_neg<O>(
    expression: &NumericNegExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> Result<ArrayImpl, ExpressionError>
where
    O: Numeric,
    for<'a> O: Scalar<RefType<'a> = O>,
    for<'a> &'a O::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> O::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    let len = validate_expression_inputs(inputs, &expression.input_types)?;
    let input = ColumnView::<O>::try_from(inputs[0].clone())?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
    for row in 0..len {
        let value = input
            .get(row)
            .map(|value| O::from_arithmetic(Neg::neg(value.into_arithmetic())));
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
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
) -> Result<ArrayImpl, ExpressionError>
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
    let len = validate_expression_inputs(inputs, &expression.input_types)?;
    let value = ColumnView::<A>::try_from(inputs[0].clone())?;
    let lower = ColumnView::<B>::try_from(inputs[1].clone())?;
    let upper = ColumnView::<C>::try_from(inputs[2].clone())?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
    for row in 0..len {
        let result = match (value.get(row), lower.get(row), upper.get(row)) {
            (Some(value), Some(lower), Some(upper)) => {
                let value = lossless_try_from::<O, A>(value);
                let lower = lossless_try_from::<O, B>(lower);
                let upper = lossless_try_from::<O, C>(upper);
                let result = if lower.partial_cmp(&upper) != Some(Ordering::Less)
                    && lower.partial_cmp(&upper) != Some(Ordering::Equal)
                {
                    Err(ScalarError::InvalidClampBounds)
                } else if value < lower {
                    Ok(lower)
                } else if value > upper {
                    Ok(upper)
                } else {
                    Ok(value)
                }
                .map_err(|error| ExpressionError::ScalarEvaluation {
                    function: expression.name,
                    row,
                    error,
                })?;
                Some(result)
            }
            _ => None,
        };
        output.push(result.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
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
) -> NumericBinaryExpression {
    let kernel = numeric_kernels(&left, &right, &output).binary;
    NumericBinaryExpression {
        name,
        input_types: [left, right],
        output_type: output,
        operator,
        kernel,
    }
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
    let kernel = numeric_kernels(&left, &right, &common).comparison;
    NumericComparisonExpression {
        name,
        input_types: [left, right],
        operator,
        kernel,
    }
}
