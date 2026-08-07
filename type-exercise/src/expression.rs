use std::any::Any;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;

use crate::column::NonNullI32Column;
use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, I32Array, PhysicalType, Scalar,
    ScalarRefImpl, TypeMismatch,
};

/// A checked failure at an expression boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionError {
    TypeMismatch(TypeMismatch),
    InputArityMismatch { expected: usize, actual: usize },
    InputLengthMismatch { left: usize, right: usize },
}

impl Display for ExpressionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeMismatch(error) => Display::fmt(error, formatter),
            Self::InputArityMismatch { expected, actual } => {
                write!(
                    formatter,
                    "input arity mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InputLengthMismatch { left, right } => {
                write!(
                    formatter,
                    "input length mismatch: left {left}, right {right}"
                )
            }
        }
    }
}

impl Error for ExpressionError {}

impl From<TypeMismatch> for ExpressionError {
    fn from(error: TypeMismatch) -> Self {
        Self::TypeMismatch(error)
    }
}

/// The loop selected for one binary evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveLoop {
    General,
    ArrayArray,
    ArrayConstant,
    ConstantArray,
    ConstantConstant,
}

/// A runtime-erased expression with discoverable physical metadata.
pub trait Expression: Any + Send + Sync {
    fn name(&self) -> &'static str;
    fn input_types(&self) -> &[PhysicalType];
    fn arity(&self) -> usize {
        self.input_types().len()
    }
    fn output_type(&self) -> PhysicalType;
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError>;
    fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<(ArrayImpl, PrimitiveLoop), ExpressionError> {
        self.evaluate(inputs)
            .map(|output| (output, PrimitiveLoop::General))
    }
}

/// One erased future for one complete batch evaluation.
pub type BatchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a>>;

/// Evaluate one borrowed batch while keeping the future type compiler-known.
#[allow(clippy::manual_async_fn)]
pub fn evaluate_static<'a, E>(
    expression: &'a E,
    inputs: &'a [ColumnViewImpl<'a>],
) -> impl Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a
where
    E: Expression + ?Sized,
{
    async move { expression.evaluate(inputs) }
}

/// A dyn-compatible asynchronous boundary around one synchronous batch evaluation.
pub trait AsyncExpression: Send + Sync {
    fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
}

/// Adapt an existing erased physical expression without changing its evaluation semantics.
pub struct AsyncExpressionAdapter {
    expression: Box<dyn Expression>,
}

impl AsyncExpressionAdapter {
    pub fn new(expression: Box<dyn Expression>) -> Self {
        Self { expression }
    }
}

impl AsyncExpression for AsyncExpressionAdapter {
    fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a> {
        Box::pin(async move { self.expression.evaluate(inputs) })
    }
}

/// One typed binary scalar function that can be lifted over nullable columns.
pub trait BinaryScalarFunction: Send + Sync + 'static {
    type Left: Scalar;
    type Right: Scalar;
    type Output: Scalar;

    fn evaluate<'a>(
        &self,
        left: <Self::Left as Scalar>::RefType<'a>,
        right: <Self::Right as Scalar>::RefType<'a>,
    ) -> Self::Output;
}

/// The Chapter 3 scalar function. Addition uses explicit wrapping semantics.
#[derive(Clone, Copy, Debug, Default)]
pub struct I32Add;

impl BinaryScalarFunction for I32Add {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate<'a>(&self, left: i32, right: i32) -> i32 {
        left.wrapping_add(right)
    }
}

/// Concatenate two borrowed strings into one owned output value.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringConcat;

impl BinaryScalarFunction for StringConcat {
    type Left = String;
    type Right = String;
    type Output = String;

    fn evaluate<'a>(&self, left: &'a str, right: &'a str) -> String {
        let mut output = String::with_capacity(left.len() + right.len());
        output.push_str(left);
        output.push_str(right);
        output
    }
}

/// Convert erased inputs once, then apply a typed scalar function row by row.
pub fn evaluate_binary<'a, F>(
    function: &F,
    left: ColumnViewImpl<'a>,
    right: ColumnViewImpl<'a>,
) -> Result<ArrayImpl, ExpressionError>
where
    F: BinaryScalarFunction,
    <F::Left as Scalar>::ArrayType: 'a,
    <F::Right as Scalar>::ArrayType: 'a,
    &'a <F::Left as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    &'a <F::Right as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    <F::Left as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    <F::Right as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    let left = ColumnView::<F::Left>::try_from(left)?;
    let right = ColumnView::<F::Right>::try_from(right)?;
    evaluate_typed_binary(function, left, right)
}

fn evaluate_typed_binary<'a, F>(
    function: &F,
    left: ColumnView<'a, F::Left>,
    right: ColumnView<'a, F::Right>,
) -> Result<ArrayImpl, ExpressionError>
where
    F: BinaryScalarFunction,
{
    if left.len() != right.len() {
        return Err(ExpressionError::InputLengthMismatch {
            left: left.len(),
            right: right.len(),
        });
    }

    let mut output =
        <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => Some(function.evaluate(left, right)),
            _ => None,
        };
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

/// An `i32` binary adapter with checked all-valid fast paths.
pub struct PrimitiveBinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}

impl<F> PrimitiveBinaryExpression<F>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32>,
{
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [PhysicalType::Int32, PhysicalType::Int32],
            function,
        }
    }

    pub fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<(ArrayImpl, PrimitiveLoop), ExpressionError> {
        if inputs.len() != self.input_types.len() {
            return Err(ExpressionError::InputArityMismatch {
                expected: self.input_types.len(),
                actual: inputs.len(),
            });
        }

        for (input, expected) in inputs.iter().zip(self.input_types) {
            if input.physical_type() != expected {
                return Err(TypeMismatch {
                    expected,
                    actual: input.physical_type(),
                }
                .into());
            }
        }

        if inputs[0].len() != inputs[1].len() {
            return Err(ExpressionError::InputLengthMismatch {
                left: inputs[0].len(),
                right: inputs[1].len(),
            });
        }

        let Some(left) = inputs[0].as_non_null_i32() else {
            return Ok((
                evaluate_binary(&self.function, inputs[0], inputs[1])?,
                PrimitiveLoop::General,
            ));
        };
        let Some(right) = inputs[1].as_non_null_i32() else {
            return Ok((
                evaluate_binary(&self.function, inputs[0], inputs[1])?,
                PrimitiveLoop::General,
            ));
        };
        debug_assert_eq!(left.len(), right.len());

        let (values, selected_loop) = match (left, right) {
            (NonNullI32Column::Array(left), NonNullI32Column::Array(right)) => (
                left.values()
                    .iter()
                    .copied()
                    .zip(right.values().iter().copied())
                    .map(|(left, right)| self.function.evaluate(left, right))
                    .collect(),
                PrimitiveLoop::ArrayArray,
            ),
            (NonNullI32Column::Array(left), NonNullI32Column::Constant { value: right, .. }) => (
                left.values()
                    .iter()
                    .copied()
                    .map(|left| self.function.evaluate(left, right))
                    .collect(),
                PrimitiveLoop::ArrayConstant,
            ),
            (NonNullI32Column::Constant { value: left, .. }, NonNullI32Column::Array(right)) => (
                right
                    .values()
                    .iter()
                    .copied()
                    .map(|right| self.function.evaluate(left, right))
                    .collect(),
                PrimitiveLoop::ConstantArray,
            ),
            (
                NonNullI32Column::Constant { value: left, len },
                NonNullI32Column::Constant { value: right, .. },
            ) => (
                (0..len)
                    .map(|_| self.function.evaluate(left, right))
                    .collect(),
                PrimitiveLoop::ConstantConstant,
            ),
        };

        Ok((I32Array::from_values(values).into(), selected_loop))
    }
}

impl<F> Expression for PrimitiveBinaryExpression<F>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32>,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> PhysicalType {
        PhysicalType::Int32
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        self.evaluate_with_loop(inputs).map(|(output, _)| output)
    }

    fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<(ArrayImpl, PrimitiveLoop), ExpressionError> {
        PrimitiveBinaryExpression::evaluate_with_loop(self, inputs)
    }
}

/// Adapt one typed binary scalar function to the runtime expression interface.
pub struct BinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}

impl<F: BinaryScalarFunction> BinaryExpression<F> {
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [F::Left::PHYSICAL_TYPE, F::Right::PHYSICAL_TYPE],
            function,
        }
    }
}

impl<F> Expression for BinaryExpression<F>
where
    F: BinaryScalarFunction,
    <F::Left as Scalar>::ArrayType: 'static,
    <F::Right as Scalar>::ArrayType: 'static,
    for<'a> &'a <F::Left as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a <F::Right as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> <F::Left as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> <F::Right as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> PhysicalType {
        F::Output::PHYSICAL_TYPE
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        if inputs.len() != self.arity() {
            return Err(ExpressionError::InputArityMismatch {
                expected: self.arity(),
                actual: inputs.len(),
            });
        }
        evaluate_binary(&self.function, inputs[0], inputs[1])
    }
}

macro_rules! define_builtin_expressions {
    ($( $name:literal => $expression:expr ),+ $(,)?) => {
        pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &[$($name),+];

        pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
            match name {
                $(
                    $name => Some(Box::new($expression)),
                )+
                _ => None,
            }
        }
    };
}

define_builtin_expressions! {
    "i32_add" => PrimitiveBinaryExpression::new("i32_add", I32Add),
    "string_concat" => BinaryExpression::new("string_concat", StringConcat),
}
