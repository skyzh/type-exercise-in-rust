#![allow(dead_code)]

use std::cmp::Ordering;
use std::num::Wrapping;
use std::ops::{Add, Mul, Neg, Sub};

use crate::{
    ArrayImpl, BinaryExpression, ColumnViewImpl, Expression, PhysicalType, Scalar, ScalarRefImpl,
    TypeMismatch, auto_vectorize_binary, evaluate_unary, try_evaluate_binary, try_evaluate_ternary,
    validate_expression_inputs,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// The first concrete scalar adapter. Its batch traversal lives in the core evaluator.
#[derive(Clone, Copy, Debug, Default)]
pub struct I32Add;

impl crate::BinaryScalarFunction for I32Add {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate<'a>(&self, left: i32, right: i32) -> i32 {
        left.wrapping_add(right)
    }
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
    + Send
    + Sync
    + 'static
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
    fn checked_divide(self, rhs: Self) -> Result<Self, &'static str>;
}

impl CheckedDivide for i16 {
    fn checked_divide(self, rhs: Self) -> Result<Self, &'static str> {
        if rhs == 0 {
            return Err("division by zero");
        }
        self.checked_div(rhs)
            .ok_or("signed integer division overflow")
    }
}

impl CheckedDivide for i32 {
    fn checked_divide(self, rhs: Self) -> Result<Self, &'static str> {
        if rhs == 0 {
            return Err("division by zero");
        }
        self.checked_div(rhs)
            .ok_or("signed integer division overflow")
    }
}

impl CheckedDivide for i64 {
    fn checked_divide(self, rhs: Self) -> Result<Self, &'static str> {
        if rhs == 0 {
            return Err("division by zero");
        }
        self.checked_div(rhs)
            .ok_or("signed integer division overflow")
    }
}

impl CheckedDivide for f32 {
    fn checked_divide(self, rhs: Self) -> Result<Self, &'static str> {
        if rhs == 0.0 {
            Err("division by zero")
        } else {
            Ok(self / rhs)
        }
    }
}

impl CheckedDivide for f64 {
    fn checked_divide(self, rhs: Self) -> Result<Self, &'static str> {
        if rhs == 0.0 {
            Err("division by zero")
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

fn divide_number<O: CheckedDivide>(left: O, right: O) -> Result<O, &'static str> {
    left.checked_divide(right)
}

fn neg_number<O: Numeric>(value: O) -> O {
    O::from_arithmetic(Neg::neg(value.into_arithmetic()))
}

fn clamp_number<O: Numeric>(value: O, lower: O, upper: O) -> Result<O, &'static str> {
    if lower.partial_cmp(&upper) != Some(Ordering::Less)
        && lower.partial_cmp(&upper) != Some(Ordering::Equal)
    {
        Err("invalid clamp bounds")
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

type NumericBinaryBatchKernel = for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
type NumericComparisonBatchKernel =
    for<'a> fn(&NumericComparisonExpression, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
type NumericNegBatchKernel =
    for<'a> fn(&NumericNegExpression, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
type NumericClampBatchKernel =
    for<'a> fn(&NumericClampExpression, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
pub struct NumericComparisonExpression {
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

impl Expression for NumericComparisonExpression {
    fn name(&self) -> &'static str {
        self.name
    }
    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }
    fn output_type(&self) -> PhysicalType {
        PhysicalType::Bool
    }
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate(inputs)
    }
}

pub struct NumericNegExpression {
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

impl Expression for NumericNegExpression {
    fn name(&self) -> &'static str {
        self.name
    }
    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }
    fn output_type(&self) -> PhysicalType {
        self.input_types[0].clone()
    }
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate(inputs)
    }
}

pub struct NumericClampExpression {
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

impl Expression for NumericClampExpression {
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
    try_evaluate_binary::<L, R, O, _, _>(inputs[0].clone(), inputs[1].clone(), "", |left, right| {
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
    fn evaluate<L, R, O, Operation>(
        inputs: &[ColumnViewImpl<'_>],
        operation: Operation,
    ) -> anyhow::Result<ArrayImpl>
    where
        L: Numeric,
        R: Numeric,
        O: Numeric
            + TryFrom<L, Error = std::convert::Infallible>
            + TryFrom<R, Error = std::convert::Infallible>,
        Operation: Fn(O, O) -> bool,
        for<'a> L: Scalar<RefType<'a> = L>,
        for<'a> R: Scalar<RefType<'a> = R>,
        for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
        for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
        for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
        for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    {
        auto_vectorize_binary::<L, R, bool, _>(
            inputs[0].clone(),
            inputs[1].clone(),
            |left, right| {
                operation(
                    lossless_try_from::<O, L>(left),
                    lossless_try_from::<O, R>(right),
                )
            },
        )
    }

    match expression.operator {
        ComparisonOperator::Less => evaluate::<L, R, O, _>(inputs, less),
        ComparisonOperator::LessOrEqual => evaluate::<L, R, O, _>(inputs, less_or_equal),
        ComparisonOperator::Greater => evaluate::<L, R, O, _>(inputs, greater),
        ComparisonOperator::GreaterOrEqual => evaluate::<L, R, O, _>(inputs, greater_or_equal),
        ComparisonOperator::Equal => evaluate::<L, R, O, _>(inputs, equal),
        ComparisonOperator::NotEqual => evaluate::<L, R, O, _>(inputs, not_equal),
    }
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
    try_evaluate_ternary::<A, B, C, O, _, _>(
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

pub fn build_numeric_neg_expression(
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

pub fn build_numeric_clamp_expression(
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
