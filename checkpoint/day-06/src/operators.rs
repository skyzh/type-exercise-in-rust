#![allow(dead_code)]

use std::cmp::Ordering;
use std::marker::PhantomData;
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

pub trait CheckedUnaryScalarFunction {
    type Input: Scalar;
    type Output: Scalar;
    fn evaluate<'a>(
        &self,
        input: <Self::Input as Scalar>::RefType<'a>,
    ) -> Result<Self::Output, ScalarError>;
}

pub trait CheckedBinaryScalarFunction {
    type Left: Scalar;
    type Right: Scalar;
    type Output: Scalar;
    fn evaluate<'a>(
        &self,
        left: <Self::Left as Scalar>::RefType<'a>,
        right: <Self::Right as Scalar>::RefType<'a>,
    ) -> Result<Self::Output, ScalarError>;
}

pub trait CheckedTernaryScalarFunction {
    type First: Scalar;
    type Second: Scalar;
    type Third: Scalar;
    type Output: Scalar;
    fn evaluate<'a>(
        &self,
        first: <Self::First as Scalar>::RefType<'a>,
        second: <Self::Second as Scalar>::RefType<'a>,
        third: <Self::Third as Scalar>::RefType<'a>,
    ) -> Result<Self::Output, ScalarError>;
}

pub struct UnaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 1],
    function: F,
}

impl<F: CheckedUnaryScalarFunction> UnaryExpression<F> {
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [F::Input::PHYSICAL_TYPE],
            function,
        }
    }
}

pub struct CheckedBinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}

impl<F: CheckedBinaryScalarFunction> CheckedBinaryExpression<F> {
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [F::Left::PHYSICAL_TYPE, F::Right::PHYSICAL_TYPE],
            function,
        }
    }
}

pub struct TernaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 3],
    function: F,
}

impl<F: CheckedTernaryScalarFunction> TernaryExpression<F> {
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [
                F::First::PHYSICAL_TYPE,
                F::Second::PHYSICAL_TYPE,
                F::Third::PHYSICAL_TYPE,
            ],
            function,
        }
    }
}

impl<F> UnaryExpression<F>
where
    F: CheckedUnaryScalarFunction,
    <F::Input as Scalar>::ArrayType: 'static,
    for<'a> &'a <F::Input as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> <F::Input as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    /// Strict concrete-loop evaluation for one checked unary function.
    ///
    /// This inherent method is the day-4-owned callable surface of the shell;
    /// the erased `Expression` boundary delegates to it from day 8 onward.
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_expression_inputs(inputs, &self.input_types)?;
        let input = ColumnView::<F::Input>::try_from(inputs[0].clone())?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match input.get(row) {
                Some(input) => Some(self.function.evaluate(input).map_err(|error| {
                    ExpressionError::ScalarEvaluation {
                        function: self.name,
                        row,
                        error,
                    }
                })?),
                None => None,
            };
            output.push(value.as_ref().map(Scalar::as_scalar_ref));
        }
        Ok(output.finish().into())
    }
}

impl<F> CheckedBinaryExpression<F>
where
    F: CheckedBinaryScalarFunction,
    <F::Left as Scalar>::ArrayType: 'static,
    <F::Right as Scalar>::ArrayType: 'static,
    for<'a> &'a <F::Left as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a <F::Right as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> <F::Left as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> <F::Right as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    /// Strict concrete-loop evaluation for one checked binary function.
    ///
    /// This inherent method is the day-4-owned callable surface of the shell;
    /// the erased `Expression` boundary delegates to it from day 8 onward.
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_expression_inputs(inputs, &self.input_types)?;
        let left = ColumnView::<F::Left>::try_from(inputs[0].clone())?;
        let right = ColumnView::<F::Right>::try_from(inputs[1].clone())?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match (left.get(row), right.get(row)) {
                (Some(left), Some(right)) => {
                    Some(self.function.evaluate(left, right).map_err(|error| {
                        ExpressionError::ScalarEvaluation {
                            function: self.name,
                            row,
                            error,
                        }
                    })?)
                }
                _ => None,
            };
            output.push(value.as_ref().map(Scalar::as_scalar_ref));
        }
        Ok(output.finish().into())
    }
}

impl<F> TernaryExpression<F>
where
    F: CheckedTernaryScalarFunction,
    <F::First as Scalar>::ArrayType: 'static,
    <F::Second as Scalar>::ArrayType: 'static,
    <F::Third as Scalar>::ArrayType: 'static,
    for<'a> &'a <F::First as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a <F::Second as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a <F::Third as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> <F::First as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> <F::Second as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> <F::Third as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_expression_inputs(inputs, &self.input_types)?;
        let first = ColumnView::<F::First>::try_from(inputs[0].clone())?;
        let second = ColumnView::<F::Second>::try_from(inputs[1].clone())?;
        let third = ColumnView::<F::Third>::try_from(inputs[2].clone())?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match (first.get(row), second.get(row), third.get(row)) {
                (Some(first), Some(second), Some(third)) => Some(
                    self.function
                        .evaluate(first, second, third)
                        .map_err(|error| ExpressionError::ScalarEvaluation {
                            function: self.name,
                            row,
                            error,
                        })?,
                ),
                _ => None,
            };
            output.push(value.as_ref().map(Scalar::as_scalar_ref));
        }
        Ok(output.finish().into())
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

trait Numeric: Scalar + Copy + PartialOrd {
    fn add(self, rhs: Self) -> Self;
    fn subtract(self, rhs: Self) -> Self;
    fn multiply(self, rhs: Self) -> Self;
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError>;
    fn negate(self) -> Self;
}

impl Numeric for i16 {
    fn add(self, rhs: Self) -> Self {
        Add::add(Wrapping(self), Wrapping(rhs)).0
    }
    fn subtract(self, rhs: Self) -> Self {
        Sub::sub(Wrapping(self), Wrapping(rhs)).0
    }
    fn multiply(self, rhs: Self) -> Self {
        Mul::mul(Wrapping(self), Wrapping(rhs)).0
    }
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0 {
            return Err(ScalarError::DivisionByZero);
        }
        self.checked_div(rhs).ok_or(ScalarError::DivisionOverflow)
    }
    fn negate(self) -> Self {
        Neg::neg(Wrapping(self)).0
    }
}

impl Numeric for i32 {
    fn add(self, rhs: Self) -> Self {
        Add::add(Wrapping(self), Wrapping(rhs)).0
    }
    fn subtract(self, rhs: Self) -> Self {
        Sub::sub(Wrapping(self), Wrapping(rhs)).0
    }
    fn multiply(self, rhs: Self) -> Self {
        Mul::mul(Wrapping(self), Wrapping(rhs)).0
    }
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0 {
            return Err(ScalarError::DivisionByZero);
        }
        self.checked_div(rhs).ok_or(ScalarError::DivisionOverflow)
    }
    fn negate(self) -> Self {
        Neg::neg(Wrapping(self)).0
    }
}

impl Numeric for i64 {
    fn add(self, rhs: Self) -> Self {
        Add::add(Wrapping(self), Wrapping(rhs)).0
    }
    fn subtract(self, rhs: Self) -> Self {
        Sub::sub(Wrapping(self), Wrapping(rhs)).0
    }
    fn multiply(self, rhs: Self) -> Self {
        Mul::mul(Wrapping(self), Wrapping(rhs)).0
    }
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0 {
            return Err(ScalarError::DivisionByZero);
        }
        self.checked_div(rhs).ok_or(ScalarError::DivisionOverflow)
    }
    fn negate(self) -> Self {
        Neg::neg(Wrapping(self)).0
    }
}

impl Numeric for f32 {
    fn add(self, rhs: Self) -> Self {
        Add::add(self, rhs)
    }
    fn subtract(self, rhs: Self) -> Self {
        Sub::sub(self, rhs)
    }
    fn multiply(self, rhs: Self) -> Self {
        Mul::mul(self, rhs)
    }
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0.0 {
            Err(ScalarError::DivisionByZero)
        } else {
            Ok(self / rhs)
        }
    }
    fn negate(self) -> Self {
        Neg::neg(self)
    }
}

impl Numeric for f64 {
    fn add(self, rhs: Self) -> Self {
        Add::add(self, rhs)
    }
    fn subtract(self, rhs: Self) -> Self {
        Sub::sub(self, rhs)
    }
    fn multiply(self, rhs: Self) -> Self {
        Mul::mul(self, rhs)
    }
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
        if rhs == 0.0 {
            Err(ScalarError::DivisionByZero)
        } else {
            Ok(self / rhs)
        }
    }
    fn negate(self) -> Self {
        Neg::neg(self)
    }
}

trait PromoteInto<T> {
    fn promote(self) -> T;
}

impl<T> PromoteInto<T> for T {
    fn promote(self) -> T {
        self
    }
}

impl PromoteInto<i32> for i16 {
    fn promote(self) -> i32 {
        self.into()
    }
}

impl PromoteInto<i64> for i16 {
    fn promote(self) -> i64 {
        self.into()
    }
}

impl PromoteInto<f32> for i16 {
    fn promote(self) -> f32 {
        self.into()
    }
}

impl PromoteInto<f64> for i16 {
    fn promote(self) -> f64 {
        self.into()
    }
}

impl PromoteInto<i64> for i32 {
    fn promote(self) -> i64 {
        self.into()
    }
}

impl PromoteInto<f64> for i32 {
    fn promote(self) -> f64 {
        self.into()
    }
}

impl PromoteInto<f64> for f32 {
    fn promote(self) -> f64 {
        self.into()
    }
}

pub(crate) struct NumericBinary<L, R, O> {
    operator: ArithmeticOperator,
    marker: PhantomData<(L, R, O)>,
}
impl<L, R, O> CheckedBinaryScalarFunction for NumericBinary<L, R, O>
where
    L: Numeric + PromoteInto<O>,
    R: Numeric + PromoteInto<O>,
    O: Numeric,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
{
    type Left = L;
    type Right = R;
    type Output = O;
    fn evaluate<'a>(&self, left: L, right: R) -> Result<O, ScalarError> {
        let left = left.promote();
        let right = right.promote();
        match self.operator {
            ArithmeticOperator::Add => Ok(left.add(right)),
            ArithmeticOperator::Subtract => Ok(left.subtract(right)),
            ArithmeticOperator::Multiply => Ok(left.multiply(right)),
            ArithmeticOperator::Divide => left.checked_divide(right),
        }
    }
}

pub(crate) struct NumericNeg<O>(PhantomData<O>);
impl<O> CheckedUnaryScalarFunction for NumericNeg<O>
where
    O: Numeric,
    for<'a> O: Scalar<RefType<'a> = O>,
{
    type Input = O;
    type Output = O;
    fn evaluate<'a>(&self, input: O) -> Result<O, ScalarError> {
        Ok(input.negate())
    }
}

pub(crate) struct NumericClamp<O>(PhantomData<O>);
impl<O> CheckedTernaryScalarFunction for NumericClamp<O>
where
    O: Numeric,
    for<'a> O: Scalar<RefType<'a> = O>,
{
    type First = O;
    type Second = O;
    type Third = O;
    type Output = O;
    fn evaluate<'a>(&self, value: O, lower: O, upper: O) -> Result<O, ScalarError> {
        if lower.partial_cmp(&upper) != Some(Ordering::Less)
            && lower.partial_cmp(&upper) != Some(Ordering::Equal)
        {
            return Err(ScalarError::InvalidClampBounds);
        }
        if value < lower {
            Ok(lower)
        } else if value > upper {
            Ok(upper)
        } else {
            Ok(value)
        }
    }
}

pub(crate) struct NumericCompare<L, R, O> {
    operator: ComparisonOperator,
    marker: PhantomData<(L, R, O)>,
}
impl<L, R, O> CheckedBinaryScalarFunction for NumericCompare<L, R, O>
where
    L: Numeric + PromoteInto<O>,
    R: Numeric + PromoteInto<O>,
    O: Numeric,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
{
    type Left = L;
    type Right = R;
    type Output = bool;
    fn evaluate<'a>(&self, left: L, right: R) -> Result<bool, ScalarError> {
        let (left, right) = (left.promote(), right.promote());
        Ok(match self.operator {
            ComparisonOperator::Less => left < right,
            ComparisonOperator::LessOrEqual => left <= right,
            ComparisonOperator::Greater => left > right,
            ComparisonOperator::GreaterOrEqual => left >= right,
            ComparisonOperator::Equal => left == right,
            ComparisonOperator::NotEqual => left != right,
        })
    }
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
type NumericCastBatchKernel = for<'a> fn(&ColumnViewImpl<'a>) -> Result<ArrayImpl, ExpressionError>;

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
    casts: [NumericCastBatchKernel; 3],
}

impl NumericClampExpression {
    pub(crate) fn evaluate(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<ArrayImpl, ExpressionError> {
        validate_expression_inputs(inputs, &self.input_types)?;
        let first = (self.casts[0])(&inputs[0])?;
        let second = (self.casts[1])(&inputs[1])?;
        let third = (self.casts[2])(&inputs[2])?;
        (self.kernel)(
            self,
            &[
                ColumnViewImpl::array(&first),
                ColumnViewImpl::array(&second),
                ColumnViewImpl::array(&third),
            ],
        )
    }
}

fn evaluate_numeric_binary<L, R, O>(
    expression: &NumericBinaryExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> Result<ArrayImpl, ExpressionError>
where
    L: Numeric + PromoteInto<O>,
    R: Numeric + PromoteInto<O>,
    O: Numeric,
    L::ArrayType: 'static,
    R::ArrayType: 'static,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    CheckedBinaryExpression::new(
        expression.name,
        NumericBinary::<L, R, O> {
            operator: expression.operator,
            marker: PhantomData,
        },
    )
    .evaluate(inputs)
}

fn evaluate_numeric_comparison<L, R, O>(
    expression: &NumericComparisonExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> Result<ArrayImpl, ExpressionError>
where
    L: Numeric + PromoteInto<O>,
    R: Numeric + PromoteInto<O>,
    O: Numeric,
    L::ArrayType: 'static,
    R::ArrayType: 'static,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    CheckedBinaryExpression::new(
        expression.name,
        NumericCompare::<L, R, O> {
            operator: expression.operator,
            marker: PhantomData,
        },
    )
    .evaluate(inputs)
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
    O::ArrayType: 'static,
    for<'a> O: Scalar<RefType<'a> = O>,
    for<'a> &'a O::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> O::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    UnaryExpression::new(expression.name, NumericNeg::<O>(PhantomData)).evaluate(inputs)
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

fn evaluate_numeric_clamp<O>(
    expression: &NumericClampExpression,
    inputs: &[ColumnViewImpl<'_>],
) -> Result<ArrayImpl, ExpressionError>
where
    O: Numeric,
    O::ArrayType: 'static,
    for<'a> O: Scalar<RefType<'a> = O>,
    for<'a> &'a O::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> O::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    TernaryExpression::new(expression.name, NumericClamp::<O>(PhantomData)).evaluate(inputs)
}

fn cast_numeric_column<L, O>(input: &ColumnViewImpl<'_>) -> Result<ArrayImpl, ExpressionError>
where
    L: Numeric + PromoteInto<O>,
    O: Numeric,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    let input = ColumnView::<L>::try_from(input.clone())?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(input.len());
    for row in 0..input.len() {
        let value = input.get(row).map(PromoteInto::promote);
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

fn numeric_cast_kernel(input: &PhysicalType, output: &PhysicalType) -> NumericCastBatchKernel {
    match (input, output) {
        (PhysicalType::Int16, PhysicalType::Int16) => cast_numeric_column::<i16, i16>,
        (PhysicalType::Int16, PhysicalType::Int32) => cast_numeric_column::<i16, i32>,
        (PhysicalType::Int16, PhysicalType::Int64) => cast_numeric_column::<i16, i64>,
        (PhysicalType::Int16, PhysicalType::Float32) => cast_numeric_column::<i16, f32>,
        (PhysicalType::Int16, PhysicalType::Float64) => cast_numeric_column::<i16, f64>,
        (PhysicalType::Int32, PhysicalType::Int32) => cast_numeric_column::<i32, i32>,
        (PhysicalType::Int32, PhysicalType::Int64) => cast_numeric_column::<i32, i64>,
        (PhysicalType::Int32, PhysicalType::Float64) => cast_numeric_column::<i32, f64>,
        (PhysicalType::Int64, PhysicalType::Int64) => cast_numeric_column::<i64, i64>,
        (PhysicalType::Float32, PhysicalType::Float32) => cast_numeric_column::<f32, f32>,
        (PhysicalType::Float32, PhysicalType::Float64) => cast_numeric_column::<f32, f64>,
        (PhysicalType::Float64, PhysicalType::Float64) => cast_numeric_column::<f64, f64>,
        _ => unreachable!("validated numeric conversion"),
    }
}

fn numeric_clamp_kernel(output: &PhysicalType) -> NumericClampBatchKernel {
    match output {
        PhysicalType::Int16 => evaluate_numeric_clamp::<i16>,
        PhysicalType::Int32 => evaluate_numeric_clamp::<i32>,
        PhysicalType::Int64 => evaluate_numeric_clamp::<i64>,
        PhysicalType::Float32 => evaluate_numeric_clamp::<f32>,
        PhysicalType::Float64 => evaluate_numeric_clamp::<f64>,
        _ => unreachable!("numeric clamp output"),
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
    let kernel = numeric_clamp_kernel(&output);
    let casts = [
        numeric_cast_kernel(&inputs[0], &output),
        numeric_cast_kernel(&inputs[1], &output),
        numeric_cast_kernel(&inputs[2], &output),
    ];
    NumericClampExpression {
        name,
        input_types: inputs,
        output_type: output,
        kernel,
        casts,
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
