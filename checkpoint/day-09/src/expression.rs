use std::any::Any;
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

pub trait Expression: Any + Send + Sync {
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

pub type BinaryBatchKernel =
    for<'a> fn(&[ColumnViewImpl<'a>]) -> Result<ArrayImpl, ExpressionError>;

pub struct BinaryExpression {
    name: &'static str,
    input_types: [crate::PhysicalType; 2],
    output_type: crate::PhysicalType,
    kernel: BinaryBatchKernel,
}

impl BinaryExpression {
    pub fn new(
        name: &'static str,
        input_types: [crate::PhysicalType; 2],
        output_type: crate::PhysicalType,
        kernel: BinaryBatchKernel,
    ) -> Self {
        Self {
            name,
            input_types,
            output_type,
            kernel,
        }
    }
}

impl Expression for BinaryExpression {
    fn name(&self) -> &'static str {
        self.name
    }

    fn input_types(&self) -> &[crate::PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> crate::PhysicalType {
        self.output_type.clone()
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        if inputs.len() != self.arity() {
            return Err(ExpressionError::InputArityMismatch {
                expected: self.arity(),
                actual: inputs.len(),
            });
        }
        for (input, expected) in inputs.iter().zip(&self.input_types) {
            if input.physical_type() != *expected {
                return Err(TypeMismatch {
                    expected: expected.clone(),
                    actual: input.physical_type(),
                }
                .into());
            }
        }
        if inputs[0].len() != inputs[1].len() {
            return Err(ExpressionError::InputLengthMismatch {
                expected: inputs[0].len(),
                actual: inputs[1].len(),
                input_index: 1,
            });
        }
        let output = (self.kernel)(inputs)?;
        if output.physical_type() != self.output_type {
            return Err(TypeMismatch {
                expected: self.output_type.clone(),
                actual: output.physical_type(),
            }
            .into());
        }
        Ok(output)
    }
}

fn evaluate_i32_add_batch(inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
    let left = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let right = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let mut output = <crate::I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        output.push(
            left.get(row)
                .zip(right.get(row))
                .map(|(left, right)| left.wrapping_add(right)),
        );
    }
    Ok(output.finish().into())
}

fn evaluate_string_concat_batch(
    inputs: &[ColumnViewImpl<'_>],
) -> Result<ArrayImpl, ExpressionError> {
    let left = ColumnView::<String>::try_from(inputs[0].clone())?;
    let right = ColumnView::<String>::try_from(inputs[1].clone())?;
    let mut output = <crate::StringArray as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = left.get(row).zip(right.get(row)).map(|(left, right)| {
            let mut output = String::with_capacity(left.len() + right.len());
            output.push_str(left);
            output.push_str(right);
            output
        });
        output.push(value.as_deref());
    }
    Ok(output.finish().into())
}

#[derive(Clone)]
struct BinaryBuiltin {
    name: &'static str,
    input_types: [crate::PhysicalType; 2],
    output_type: crate::PhysicalType,
    kernel: BinaryBatchKernel,
}

impl BinaryBuiltin {
    fn build(&self) -> BinaryExpression {
        BinaryExpression::new(
            self.name,
            self.input_types.clone(),
            self.output_type.clone(),
            self.kernel,
        )
    }
}

macro_rules! define_builtin_expressions {
    ($( $name:literal => ($left:expr, $right:expr) -> $output:expr => $kernel:path ),+ $(,)?) => {
        pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &[$($name),+];

        const BUILTIN_EXPRESSIONS: &[BinaryBuiltin] = &[
            $(BinaryBuiltin {
                name: $name,
                input_types: [$left, $right],
                output_type: $output,
                kernel: $kernel,
            }),+
        ];

        pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
            BUILTIN_EXPRESSIONS
                .iter()
                .find(|builtin| builtin.name == name)
                .map(|builtin| Box::new(builtin.build()) as Box<dyn Expression>)
        }
    };
}

define_builtin_expressions! {
    "i32_add" => (crate::PhysicalType::Int32, crate::PhysicalType::Int32)
        -> crate::PhysicalType::Int32 => evaluate_i32_add_batch,
    "string_concat" => (crate::PhysicalType::String, crate::PhysicalType::String)
        -> crate::PhysicalType::String => evaluate_string_concat_batch,
}
