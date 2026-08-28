#![allow(dead_code)]

use std::num::Wrapping;
use std::ops::{Add, Mul, Sub};

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
{
    type Arithmetic: Add<Output = Self::Arithmetic>
        + Sub<Output = Self::Arithmetic>
        + Mul<Output = Self::Arithmetic>;

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
