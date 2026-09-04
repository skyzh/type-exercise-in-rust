use std::cmp::Ordering;
use std::num::Wrapping;
use std::ops::{Add, Mul, Neg, Sub};

use crate::{
    ArrayImpl, BatchExpression, BatchKernel, ColumnViewImpl, PhysicalType, Scalar, ScalarRefImpl,
    TypeMismatch, auto_vectorize_binary, auto_vectorize_primitive_i32, auto_vectorize_unary,
    try_auto_vectorize_ternary, try_evaluate_binary,
};

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

type NumericBinaryBatchKernel = BatchKernel;
type NumericComparisonBatchKernel = BatchKernel;
type NumericNegBatchKernel = BatchKernel;
type NumericClampBatchKernel = BatchKernel;

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
    try_evaluate_binary::<L, R, O, _, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        "numeric_divide",
        |left, right| {
            let left = lossless_try_from::<O, L>(left);
            let right = lossless_try_from::<O, R>(right);
            divide_number(left, right)
        },
    )
}

fn evaluate_i32_add(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_primitive_i32(inputs[0].clone(), inputs[1].clone(), i32::wrapping_add)
}

fn evaluate_i32_subtract(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_primitive_i32(inputs[0].clone(), inputs[1].clone(), i32::wrapping_sub)
}

fn evaluate_i32_multiply(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_primitive_i32(inputs[0].clone(), inputs[1].clone(), i32::wrapping_mul)
}

fn primitive_i32_kernel(operator: ArithmeticOperator) -> NumericBinaryBatchKernel {
    match operator {
        ArithmeticOperator::Add => evaluate_i32_add,
        ArithmeticOperator::Subtract => evaluate_i32_subtract,
        ArithmeticOperator::Multiply => evaluate_i32_multiply,
        ArithmeticOperator::Divide => evaluate_numeric_divide::<i32, i32, i32>,
    }
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

fn evaluate_numeric_comparison<L, R, O, Operation>(
    inputs: &[ColumnViewImpl<'_>],
    operation: Operation,
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
    Operation: Fn(O, O) -> bool,
{
    auto_vectorize_binary::<L, R, bool, _>(inputs[0].clone(), inputs[1].clone(), |left, right| {
        operation(
            lossless_try_from::<O, L>(left),
            lossless_try_from::<O, R>(right),
        )
    })
}

macro_rules! define_numeric_comparison_kernel {
    ($name:ident, $operation:ident) => {
        fn $name<L, R, O>(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>
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
            evaluate_numeric_comparison::<L, R, O, _>(inputs, $operation)
        }
    };
}

define_numeric_comparison_kernel!(evaluate_numeric_less, less);
define_numeric_comparison_kernel!(evaluate_numeric_less_or_equal, less_or_equal);
define_numeric_comparison_kernel!(evaluate_numeric_greater, greater);
define_numeric_comparison_kernel!(evaluate_numeric_greater_or_equal, greater_or_equal);
define_numeric_comparison_kernel!(evaluate_numeric_equal, equal);
define_numeric_comparison_kernel!(evaluate_numeric_not_equal, not_equal);

struct NumericComparisonKernels {
    less: NumericComparisonBatchKernel,
    less_or_equal: NumericComparisonBatchKernel,
    greater: NumericComparisonBatchKernel,
    greater_or_equal: NumericComparisonBatchKernel,
    equal: NumericComparisonBatchKernel,
    not_equal: NumericComparisonBatchKernel,
}

impl NumericComparisonKernels {
    fn select(&self, operator: ComparisonOperator) -> NumericComparisonBatchKernel {
        match operator {
            ComparisonOperator::Less => self.less,
            ComparisonOperator::LessOrEqual => self.less_or_equal,
            ComparisonOperator::Greater => self.greater,
            ComparisonOperator::GreaterOrEqual => self.greater_or_equal,
            ComparisonOperator::Equal => self.equal,
            ComparisonOperator::NotEqual => self.not_equal,
        }
    }
}

fn numeric_comparison_kernels<L, R, O>() -> NumericComparisonKernels
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
    NumericComparisonKernels {
        less: evaluate_numeric_less::<L, R, O>,
        less_or_equal: evaluate_numeric_less_or_equal::<L, R, O>,
        greater: evaluate_numeric_greater::<L, R, O>,
        greater_or_equal: evaluate_numeric_greater_or_equal::<L, R, O>,
        equal: evaluate_numeric_equal::<L, R, O>,
        not_equal: evaluate_numeric_not_equal::<L, R, O>,
    }
}

struct NumericKernels {
    binary: NumericBinaryBatchKernel,
    comparisons: NumericComparisonKernels,
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
            comparisons: numeric_comparison_kernels::<i16, i16, i16>(),
        },
        (PhysicalType::Int16, PhysicalType::Int32, PhysicalType::Int32) => NumericKernels {
            binary: numeric_binary_kernel::<i16, i32, i32>(operator),
            comparisons: numeric_comparison_kernels::<i16, i32, i32>(),
        },
        (PhysicalType::Int32, PhysicalType::Int16, PhysicalType::Int32) => NumericKernels {
            binary: numeric_binary_kernel::<i32, i16, i32>(operator),
            comparisons: numeric_comparison_kernels::<i32, i16, i32>(),
        },
        (PhysicalType::Int16, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i16, i64, i64>(operator),
            comparisons: numeric_comparison_kernels::<i16, i64, i64>(),
        },
        (PhysicalType::Int64, PhysicalType::Int16, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i64, i16, i64>(operator),
            comparisons: numeric_comparison_kernels::<i64, i16, i64>(),
        },
        (PhysicalType::Int16, PhysicalType::Float32, PhysicalType::Float32) => NumericKernels {
            binary: numeric_binary_kernel::<i16, f32, f32>(operator),
            comparisons: numeric_comparison_kernels::<i16, f32, f32>(),
        },
        (PhysicalType::Float32, PhysicalType::Int16, PhysicalType::Float32) => NumericKernels {
            binary: numeric_binary_kernel::<f32, i16, f32>(operator),
            comparisons: numeric_comparison_kernels::<f32, i16, f32>(),
        },
        (PhysicalType::Int16, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<i16, f64, f64>(operator),
            comparisons: numeric_comparison_kernels::<i16, f64, f64>(),
        },
        (PhysicalType::Float64, PhysicalType::Int16, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f64, i16, f64>(operator),
            comparisons: numeric_comparison_kernels::<f64, i16, f64>(),
        },
        (PhysicalType::Int32, PhysicalType::Int32, PhysicalType::Int32) => NumericKernels {
            binary: primitive_i32_kernel(operator),
            comparisons: numeric_comparison_kernels::<i32, i32, i32>(),
        },
        (PhysicalType::Int32, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i32, i64, i64>(operator),
            comparisons: numeric_comparison_kernels::<i32, i64, i64>(),
        },
        (PhysicalType::Int64, PhysicalType::Int32, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i64, i32, i64>(operator),
            comparisons: numeric_comparison_kernels::<i64, i32, i64>(),
        },
        (PhysicalType::Int32, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<i32, f64, f64>(operator),
            comparisons: numeric_comparison_kernels::<i32, f64, f64>(),
        },
        (PhysicalType::Int32, PhysicalType::Float32, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<i32, f32, f64>(operator),
            comparisons: numeric_comparison_kernels::<i32, f32, f64>(),
        },
        (PhysicalType::Float32, PhysicalType::Int32, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f32, i32, f64>(operator),
            comparisons: numeric_comparison_kernels::<f32, i32, f64>(),
        },
        (PhysicalType::Float64, PhysicalType::Int32, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f64, i32, f64>(operator),
            comparisons: numeric_comparison_kernels::<f64, i32, f64>(),
        },
        (PhysicalType::Int64, PhysicalType::Int64, PhysicalType::Int64) => NumericKernels {
            binary: numeric_binary_kernel::<i64, i64, i64>(operator),
            comparisons: numeric_comparison_kernels::<i64, i64, i64>(),
        },
        (PhysicalType::Float32, PhysicalType::Float32, PhysicalType::Float32) => NumericKernels {
            binary: numeric_binary_kernel::<f32, f32, f32>(operator),
            comparisons: numeric_comparison_kernels::<f32, f32, f32>(),
        },
        (PhysicalType::Float32, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f32, f64, f64>(operator),
            comparisons: numeric_comparison_kernels::<f32, f64, f64>(),
        },
        (PhysicalType::Float64, PhysicalType::Float32, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f64, f32, f64>(operator),
            comparisons: numeric_comparison_kernels::<f64, f32, f64>(),
        },
        (PhysicalType::Float64, PhysicalType::Float64, PhysicalType::Float64) => NumericKernels {
            binary: numeric_binary_kernel::<f64, f64, f64>(operator),
            comparisons: numeric_comparison_kernels::<f64, f64, f64>(),
        },
        _ => unreachable!("validated numeric promotion tuple"),
    }
}

fn evaluate_numeric_neg<O>(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>
where
    O: Numeric,
    for<'a> O: Scalar<RefType<'a> = O>,
    for<'a> &'a O::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> O::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    auto_vectorize_unary::<O, O, _>(inputs[0].clone(), neg_number::<O>)
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

fn evaluate_numeric_clamp<A, B, C, O>(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>
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
    try_auto_vectorize_ternary::<A, B, C, O, _, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        inputs[2].clone(),
        "numeric_clamp",
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
) -> BatchExpression<2> {
    let kernel = numeric_kernels(operator, &left, &right, &output).binary;
    BatchExpression::new(name, [left, right], output, kernel)
}

pub(crate) fn build_numeric_neg_expression(
    name: &'static str,
    input: PhysicalType,
) -> BatchExpression<1> {
    let kernel = numeric_neg_kernel(&input);
    BatchExpression::new(name, [input.clone()], input, kernel)
}

pub(crate) fn build_numeric_clamp_expression(
    name: &'static str,
    inputs: [PhysicalType; 3],
    output: PhysicalType,
) -> BatchExpression<3> {
    let kernel = numeric_clamp_kernel(&inputs, &output);
    BatchExpression::new(name, inputs, output, kernel)
}

pub(crate) fn build_numeric_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
    left: PhysicalType,
    right: PhysicalType,
    common: PhysicalType,
) -> BatchExpression<2> {
    let kernel = numeric_kernels(ArithmeticOperator::Add, &left, &right, &common)
        .comparisons
        .select(operator);
    BatchExpression::new(name, [left, right], PhysicalType::Bool, kernel)
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
