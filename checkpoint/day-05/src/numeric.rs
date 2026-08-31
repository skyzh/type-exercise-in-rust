#![allow(dead_code)]

use std::num::Wrapping;
use std::ops::{Add, Mul, Sub};

use crate::{
    ArrayImpl, BinaryExpression, BinaryScalarFunction, ColumnViewImpl, Expression, PhysicalType,
    Scalar, ScalarRefImpl, TypeMismatch, auto_vectorize_binary, try_auto_vectorize_binary,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct I32Add;

impl BinaryScalarFunction for I32Add {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate(&self, left: i32, right: i32) -> i32 {
        left.wrapping_add(right)
    }
}

pub(crate) fn validate_expression_inputs(
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
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

fn lossless_try_from<T, U>(value: U) -> T
where
    T: TryFrom<U, Error = std::convert::Infallible>,
{
    T::try_from(value).unwrap_or_else(|never| match never {})
}

type NumericBinaryBatchKernel = for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
type NumericComparisonBatchKernel =
    for<'a> fn(&NumericComparisonExpression, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
pub struct NumericComparisonExpression {
    name: &'static str,
    input_types: [PhysicalType; 2],
    operator: ComparisonOperator,
    kernel: NumericComparisonBatchKernel,
}

impl NumericComparisonExpression {
    pub(crate) fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        (self.kernel)(self, inputs)
    }
}

impl Expression for NumericComparisonExpression {
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate(inputs)
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
    Operation: Fn(O::Arithmetic, O::Arithmetic) -> O::Arithmetic,
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
        O::from_arithmetic(operation(left.into_arithmetic(), right.into_arithmetic()))
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
    evaluate_numeric_infallible::<L, R, O, _>(inputs, Add::add)
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
    evaluate_numeric_infallible::<L, R, O, _>(inputs, Sub::sub)
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
    evaluate_numeric_infallible::<L, R, O, _>(inputs, Mul::mul)
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
    try_auto_vectorize_binary::<L, R, O, _>(inputs[0].clone(), inputs[1].clone(), |left, right| {
        let left = lossless_try_from::<O, L>(left);
        let right = lossless_try_from::<O, R>(right);
        left.checked_divide(right)
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
    let operation: fn(O, O) -> bool = match expression.operator {
        ComparisonOperator::Less => less::<O>,
        ComparisonOperator::LessOrEqual => less_or_equal::<O>,
        ComparisonOperator::Greater => greater::<O>,
        ComparisonOperator::GreaterOrEqual => greater_or_equal::<O>,
        ComparisonOperator::Equal => equal::<O>,
        ComparisonOperator::NotEqual => not_equal::<O>,
    };
    validate_expression_inputs(inputs, &expression.input_types)?;
    auto_vectorize_binary::<L, R, bool, _>(inputs[0].clone(), inputs[1].clone(), |left, right| {
        let left = lossless_try_from::<O, L>(left);
        let right = lossless_try_from::<O, R>(right);
        operation(left, right)
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

pub fn build_numeric_binary_expression(
    name: &'static str,
    operator: ArithmeticOperator,
    left: PhysicalType,
    right: PhysicalType,
    output: PhysicalType,
) -> BinaryExpression {
    let kernel = numeric_kernels(operator, &left, &right, &output).binary;
    BinaryExpression::new_with_scalar_rows(name, [left, right], output, kernel)
}

pub fn build_numeric_comparison_expression(
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonOperator {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

pub(crate) fn less<T: PartialOrd>(left: T, right: T) -> bool {
    left < right
}

pub(crate) fn less_or_equal<T: PartialOrd>(left: T, right: T) -> bool {
    left <= right
}

pub(crate) fn greater<T: PartialOrd>(left: T, right: T) -> bool {
    left > right
}

pub(crate) fn greater_or_equal<T: PartialOrd>(left: T, right: T) -> bool {
    left >= right
}

pub(crate) fn equal<T: PartialEq>(left: T, right: T) -> bool {
    left == right
}

pub(crate) fn not_equal<T: PartialEq>(left: T, right: T) -> bool {
    left != right
}
