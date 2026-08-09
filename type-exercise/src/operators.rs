use std::cmp::Ordering;
use std::marker::PhantomData;

use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnViewImpl, Expression, ExpressionError, PhysicalType,
    Scalar, ScalarError, ScalarRefImpl, TypeMismatch,
};

/// Validate an expression boundary completely before row access or output allocation.
///
/// Physical operator constructors are deliberately binder-internal:
///
/// ```compile_fail
/// use type_exercise::build_numeric_binary_expression;
/// ```
///
/// ```compile_fail
/// use type_exercise::build_numeric_neg_expression;
/// ```
///
/// ```compile_fail
/// use type_exercise::build_numeric_clamp_expression;
/// ```
///
/// ```compile_fail
/// use type_exercise::build_numeric_comparison_expression;
/// ```
///
/// ```compile_fail
/// use type_exercise::build_string_comparison_expression;
/// ```
///
/// ```compile_fail
/// use type_exercise::build_string_contains_expression;
/// ```
///
/// ```compile_fail
/// use type_exercise::build_bool_comparison_expression;
/// ```
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

pub trait CheckedUnaryScalarFunction: Send + Sync + 'static {
    type Output: Scalar;
    fn evaluate(&self, input: ScalarRefImpl<'_>) -> Result<Self::Output, ScalarError>;
}

pub trait CheckedBinaryScalarFunction: Send + Sync + 'static {
    type Output: Scalar;
    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<Self::Output, ScalarError>;
}

pub trait CheckedTernaryScalarFunction: Send + Sync + 'static {
    type Output: Scalar;
    fn evaluate(
        &self,
        first: ScalarRefImpl<'_>,
        second: ScalarRefImpl<'_>,
        third: ScalarRefImpl<'_>,
    ) -> Result<Self::Output, ScalarError>;
}

macro_rules! expression_shell {
    ($expression:ident, $arity:literal) => {
        pub struct $expression<F> {
            name: &'static str,
            input_types: [PhysicalType; $arity],
            function: F,
        }
        impl<F> $expression<F> {
            pub fn new(
                name: &'static str,
                input_types: [PhysicalType; $arity],
                function: F,
            ) -> Self {
                Self {
                    name,
                    input_types,
                    function,
                }
            }
        }
    };
}

expression_shell!(UnaryExpression, 1);
expression_shell!(CheckedBinaryExpression, 2);
expression_shell!(TernaryExpression, 3);

macro_rules! expression_metadata {
    ($function:ident) => {
        fn name(&self) -> &'static str {
            self.name
        }
        fn input_types(&self) -> &[PhysicalType] {
            &self.input_types
        }
        fn output_type(&self) -> PhysicalType {
            F::Output::PHYSICAL_TYPE
        }
    };
}

impl<F: CheckedUnaryScalarFunction> Expression for UnaryExpression<F> {
    expression_metadata!(CheckedUnaryScalarFunction);
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_expression_inputs(inputs, &self.input_types)?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match inputs[0].get(row) {
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

impl<F: CheckedBinaryScalarFunction> Expression for CheckedBinaryExpression<F> {
    expression_metadata!(CheckedBinaryScalarFunction);
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_expression_inputs(inputs, &self.input_types)?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match (inputs[0].get(row), inputs[1].get(row)) {
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

impl<F: CheckedTernaryScalarFunction> Expression for TernaryExpression<F> {
    expression_metadata!(CheckedTernaryScalarFunction);
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_expression_inputs(inputs, &self.input_types)?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match (inputs[0].get(row), inputs[1].get(row), inputs[2].get(row)) {
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

trait Numeric: Scalar + Copy + PartialOrd + Send + Sync + 'static {
    fn from_erased(value: ScalarRefImpl<'_>) -> Self;
    fn wrapping_add(self, rhs: Self) -> Self;
    fn wrapping_sub(self, rhs: Self) -> Self;
    fn wrapping_mul(self, rhs: Self) -> Self;
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError>;
    fn wrapping_neg(self) -> Self;
}

macro_rules! impl_integer_numeric {
    ($type:ty, $variant:ident) => {
        impl Numeric for $type {
            fn from_erased(value: ScalarRefImpl<'_>) -> Self {
                match value {
                    ScalarRefImpl::Int16(value) => value as Self,
                    ScalarRefImpl::Int32(value) => value as Self,
                    ScalarRefImpl::Int64(value) => value as Self,
                    _ => unreachable!("validated numeric input"),
                }
            }
            fn wrapping_add(self, rhs: Self) -> Self {
                self.wrapping_add(rhs)
            }
            fn wrapping_sub(self, rhs: Self) -> Self {
                self.wrapping_sub(rhs)
            }
            fn wrapping_mul(self, rhs: Self) -> Self {
                self.wrapping_mul(rhs)
            }
            fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
                if rhs == 0 {
                    return Err(ScalarError::DivisionByZero);
                }
                self.checked_div(rhs).ok_or(ScalarError::DivisionOverflow)
            }
            fn wrapping_neg(self) -> Self {
                self.wrapping_neg()
            }
        }
    };
}

impl_integer_numeric!(i16, Int16);
impl_integer_numeric!(i32, Int32);
impl_integer_numeric!(i64, Int64);

macro_rules! impl_float_numeric {
    ($type:ty) => {
        impl Numeric for $type {
            fn from_erased(value: ScalarRefImpl<'_>) -> Self {
                match value {
                    ScalarRefImpl::Int16(value) => value as Self,
                    ScalarRefImpl::Int32(value) => value as Self,
                    ScalarRefImpl::Float32(value) => value as Self,
                    ScalarRefImpl::Float64(value) => value as Self,
                    _ => unreachable!("validated numeric input"),
                }
            }
            fn wrapping_add(self, rhs: Self) -> Self {
                self + rhs
            }
            fn wrapping_sub(self, rhs: Self) -> Self {
                self - rhs
            }
            fn wrapping_mul(self, rhs: Self) -> Self {
                self * rhs
            }
            fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError> {
                if rhs == 0.0 {
                    Err(ScalarError::DivisionByZero)
                } else {
                    Ok(self / rhs)
                }
            }
            fn wrapping_neg(self) -> Self {
                -self
            }
        }
    };
}

impl_float_numeric!(f32);
impl_float_numeric!(f64);

struct NumericBinary<O> {
    operator: ArithmeticOperator,
    marker: PhantomData<O>,
}
impl<O> CheckedBinaryScalarFunction for NumericBinary<O>
where
    O: Numeric,
{
    type Output = O;
    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<O, ScalarError> {
        let left = O::from_erased(left);
        let right = O::from_erased(right);
        match self.operator {
            ArithmeticOperator::Add => Ok(left.wrapping_add(right)),
            ArithmeticOperator::Subtract => Ok(left.wrapping_sub(right)),
            ArithmeticOperator::Multiply => Ok(left.wrapping_mul(right)),
            ArithmeticOperator::Divide => left.checked_divide(right),
        }
    }
}

struct NumericNeg<O>(PhantomData<O>);
impl<O: Numeric> CheckedUnaryScalarFunction for NumericNeg<O> {
    type Output = O;
    fn evaluate(&self, input: ScalarRefImpl<'_>) -> Result<O, ScalarError> {
        Ok(O::from_erased(input).wrapping_neg())
    }
}

struct NumericClamp<O>(PhantomData<O>);
impl<O: Numeric> CheckedTernaryScalarFunction for NumericClamp<O> {
    type Output = O;
    fn evaluate(
        &self,
        value: ScalarRefImpl<'_>,
        lower: ScalarRefImpl<'_>,
        upper: ScalarRefImpl<'_>,
    ) -> Result<O, ScalarError> {
        let (value, lower, upper) = (
            O::from_erased(value),
            O::from_erased(lower),
            O::from_erased(upper),
        );
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

struct NumericCompare<O> {
    operator: ComparisonOperator,
    marker: PhantomData<O>,
}
impl<O: Numeric> CheckedBinaryScalarFunction for NumericCompare<O> {
    type Output = bool;
    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<bool, ScalarError> {
        let (left, right) = (O::from_erased(left), O::from_erased(right));
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

struct StringBinary {
    operator: StringOperator,
}
enum StringOperator {
    Compare(ComparisonOperator),
    Contains,
}
impl CheckedBinaryScalarFunction for StringBinary {
    type Output = bool;
    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<bool, ScalarError> {
        let (ScalarRefImpl::String(left), ScalarRefImpl::String(right)) = (left, right) else {
            unreachable!("validated string input")
        };
        Ok(match self.operator {
            StringOperator::Contains => left.contains(right),
            StringOperator::Compare(operator) => match operator {
                ComparisonOperator::Less => left < right,
                ComparisonOperator::LessOrEqual => left <= right,
                ComparisonOperator::Greater => left > right,
                ComparisonOperator::GreaterOrEqual => left >= right,
                ComparisonOperator::Equal => left == right,
                ComparisonOperator::NotEqual => left != right,
            },
        })
    }
}

struct BoolCompare(ComparisonOperator);
impl CheckedBinaryScalarFunction for BoolCompare {
    type Output = bool;
    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<bool, ScalarError> {
        let (ScalarRefImpl::Bool(left), ScalarRefImpl::Bool(right)) = (left, right) else {
            unreachable!("validated boolean input")
        };
        Ok(match self.0 {
            ComparisonOperator::Equal => left == right,
            ComparisonOperator::NotEqual => left != right,
            _ => unreachable!("ordered boolean comparison"),
        })
    }
}

macro_rules! dispatch_numeric_output {
    ($output:expr, $constructor:expr) => {
        match $output {
            PhysicalType::Int16 => {
                Box::new($constructor(PhantomData::<i16>)) as Box<dyn Expression>
            }
            PhysicalType::Int32 => Box::new($constructor(PhantomData::<i32>)),
            PhysicalType::Int64 => Box::new($constructor(PhantomData::<i64>)),
            PhysicalType::Float32 => Box::new($constructor(PhantomData::<f32>)),
            PhysicalType::Float64 => Box::new($constructor(PhantomData::<f64>)),
            _ => unreachable!("numeric promotion output"),
        }
    };
}

pub(crate) fn build_numeric_binary_expression(
    name: &'static str,
    operator: ArithmeticOperator,
    left: PhysicalType,
    right: PhysicalType,
    output: PhysicalType,
) -> Box<dyn Expression> {
    let inputs = [left, right];
    dispatch_numeric_output!(output, |marker| CheckedBinaryExpression::new(
        name,
        inputs,
        NumericBinary { operator, marker }
    ))
}

pub(crate) fn build_numeric_neg_expression(
    name: &'static str,
    input: PhysicalType,
) -> Box<dyn Expression> {
    dispatch_numeric_output!(input, |marker| UnaryExpression::new(
        name,
        [input],
        NumericNeg(marker)
    ))
}

pub(crate) fn build_numeric_clamp_expression(
    name: &'static str,
    inputs: [PhysicalType; 3],
    output: PhysicalType,
) -> Box<dyn Expression> {
    dispatch_numeric_output!(output, |marker| TernaryExpression::new(
        name,
        inputs,
        NumericClamp(marker)
    ))
}

pub(crate) fn build_numeric_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
    left: PhysicalType,
    right: PhysicalType,
    common: PhysicalType,
) -> Box<dyn Expression> {
    let inputs = [left, right];
    dispatch_numeric_output!(common, |marker| CheckedBinaryExpression::new(
        name,
        inputs,
        NumericCompare { operator, marker }
    ))
}

pub(crate) fn build_string_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
) -> Box<dyn Expression> {
    Box::new(CheckedBinaryExpression::new(
        name,
        [const { PhysicalType::String }; 2],
        StringBinary {
            operator: StringOperator::Compare(operator),
        },
    ))
}

pub(crate) fn build_string_contains_expression(name: &'static str) -> Box<dyn Expression> {
    Box::new(CheckedBinaryExpression::new(
        name,
        [const { PhysicalType::String }; 2],
        StringBinary {
            operator: StringOperator::Contains,
        },
    ))
}

pub(crate) fn build_bool_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
) -> Box<dyn Expression> {
    Box::new(CheckedBinaryExpression::new(
        name,
        [const { PhysicalType::Bool }; 2],
        BoolCompare(operator),
    ))
}
