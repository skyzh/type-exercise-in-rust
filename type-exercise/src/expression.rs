use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, PhysicalType, Scalar,
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

/// A runtime-erased expression with discoverable physical metadata.
pub trait Expression {
    fn name(&self) -> &'static str;
    fn input_types(&self) -> &[PhysicalType];
    fn arity(&self) -> usize {
        self.input_types().len()
    }
    fn output_type(&self) -> PhysicalType;
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError>;
}

/// One typed binary scalar function that can be lifted over nullable columns.
pub trait BinaryScalarFunction {
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
    ($( $name:literal => $function:expr ),+ $(,)?) => {
        pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &[$($name),+];

        pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
            match name {
                $(
                    $name => Some(Box::new(BinaryExpression::new($name, $function))),
                )+
                _ => None,
            }
        }
    };
}

define_builtin_expressions! {
    "i32_add" => I32Add,
    "string_concat" => StringConcat,
}
