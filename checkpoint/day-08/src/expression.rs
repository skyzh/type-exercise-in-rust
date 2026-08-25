use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, Scalar, ScalarRefImpl, TypeMismatch,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpressionError {
    TypeMismatch(TypeMismatch),
    InputArityMismatch {
        expected: usize,
        actual: usize,
    },
    InputLengthMismatch {
        expected: usize,
        actual: usize,
        input_index: usize,
    },
    ScalarEvaluation {
        function: &'static str,
        row: usize,
        error: ScalarError,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarError {
    DivisionByZero,
    DivisionOverflow,
    InvalidClampBounds,
}

impl Display for ScalarError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DivisionByZero => formatter.write_str("division by zero"),
            Self::DivisionOverflow => formatter.write_str("signed integer division overflow"),
            Self::InvalidClampBounds => formatter.write_str("invalid clamp bounds"),
        }
    }
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
            Self::InputLengthMismatch {
                expected,
                actual,
                input_index,
            } => write!(
                formatter,
                "input {input_index} length mismatch: expected {expected}, got {actual}"
            ),
            Self::ScalarEvaluation {
                function,
                row,
                error,
            } => write!(
                formatter,
                "function `{function}` failed at row {row}: {error}"
            ),
        }
    }
}

impl Error for ExpressionError {}

impl From<TypeMismatch> for ExpressionError {
    fn from(error: TypeMismatch) -> Self {
        Self::TypeMismatch(error)
    }
}

pub trait Expression {
    fn name(&self) -> &'static str;
    fn input_types(&self) -> &[crate::PhysicalType];
    fn arity(&self) -> usize {
        self.input_types().len()
    }
    fn output_type(&self) -> crate::PhysicalType;
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError>;
}

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
            expected: left.len(),
            actual: right.len(),
            input_index: 1,
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

pub struct BinaryExpression<F> {
    name: &'static str,
    input_types: [crate::PhysicalType; 2],
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

    fn input_types(&self) -> &[crate::PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> crate::PhysicalType {
        F::Output::PHYSICAL_TYPE
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        if inputs.len() != self.arity() {
            return Err(ExpressionError::InputArityMismatch {
                expected: self.arity(),
                actual: inputs.len(),
            });
        }
        evaluate_binary(&self.function, inputs[0].clone(), inputs[1].clone())
    }
}

pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &["i32_add", "string_concat"];

pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
    match name {
        "i32_add" => Some(Box::new(BinaryExpression::new("i32_add", I32Add))),
        "string_concat" => Some(Box::new(BinaryExpression::new(
            "string_concat",
            StringConcat,
        ))),
        _ => None,
    }
}
