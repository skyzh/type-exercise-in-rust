use std::cmp::Ordering;
use std::marker::PhantomData;

use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnViewImpl, Expression, ExpressionError, PhysicalType,
    Scalar, ScalarError, ScalarRefImpl, TypeMismatch,
};

pub trait CheckedUnaryScalarFunction {
    type Output: Scalar;
    fn evaluate(&self, input: ScalarRefImpl<'_>) -> Result<Self::Output, ScalarError>;
}

pub trait CheckedBinaryScalarFunction {
    type Output: Scalar;
    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<Self::Output, ScalarError>;
}

pub trait CheckedTernaryScalarFunction {
    type Output: Scalar;
    fn evaluate(
        &self,
        first: ScalarRefImpl<'_>,
        second: ScalarRefImpl<'_>,
        third: ScalarRefImpl<'_>,
    ) -> Result<Self::Output, ScalarError>;
}

pub struct UnaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 1],
    function: F,
}

impl<F> UnaryExpression<F> {
    pub fn new(name: &'static str, input_types: [PhysicalType; 1], function: F) -> Self {
        Self {
            name,
            input_types,
            function,
        }
    }
}

pub struct CheckedBinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}

pub struct TernaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 3],
    function: F,
}

impl<F> TernaryExpression<F> {
    pub fn new(name: &'static str, input_types: [PhysicalType; 3], function: F) -> Self {
        Self {
            name,
            input_types,
            function,
        }
    }
}

impl<F> CheckedBinaryExpression<F> {
    pub fn new(name: &'static str, input_types: [PhysicalType; 2], function: F) -> Self {
        Self {
            name,
            input_types,
            function,
        }
    }
}

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

impl<F: CheckedUnaryScalarFunction> UnaryExpression<F> {
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
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

impl<F: CheckedBinaryScalarFunction> CheckedBinaryExpression<F> {
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
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

impl<F: CheckedTernaryScalarFunction> TernaryExpression<F> {
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
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

impl<F: CheckedUnaryScalarFunction> Expression for UnaryExpression<F> {
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
        self.evaluate(inputs)
    }
}

impl<F: CheckedBinaryScalarFunction> Expression for CheckedBinaryExpression<F> {
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
        self.evaluate(inputs)
    }
}

impl<F: CheckedTernaryScalarFunction> Expression for TernaryExpression<F> {
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
        self.evaluate(inputs)
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
    fn from_erased(value: ScalarRefImpl<'_>) -> Self;
    fn wrapping_add(self, rhs: Self) -> Self;
    fn wrapping_sub(self, rhs: Self) -> Self;
    fn wrapping_mul(self, rhs: Self) -> Self;
    fn checked_divide(self, rhs: Self) -> Result<Self, ScalarError>;
    fn wrapping_neg(self) -> Self;
}

macro_rules! impl_integer_numeric {
    ($type:ty) => {
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

impl_integer_numeric!(i16);
impl_integer_numeric!(i32);
impl_integer_numeric!(i64);

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

pub(crate) struct NumericBinary<O> {
    operator: ArithmeticOperator,
    marker: PhantomData<O>,
}

impl<O: Numeric> CheckedBinaryScalarFunction for NumericBinary<O> {
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

pub(crate) struct NumericCompare<O> {
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
        let left = O::from_erased(left);
        let right = O::from_erased(right);
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

pub(crate) struct NumericNeg<O>(PhantomData<O>);

impl<O: Numeric> CheckedUnaryScalarFunction for NumericNeg<O> {
    type Output = O;

    fn evaluate(&self, input: ScalarRefImpl<'_>) -> Result<O, ScalarError> {
        Ok(O::from_erased(input).wrapping_neg())
    }
}

pub(crate) struct NumericClamp<O>(PhantomData<O>);

impl<O: Numeric> CheckedTernaryScalarFunction for NumericClamp<O> {
    type Output = O;

    fn evaluate(
        &self,
        value: ScalarRefImpl<'_>,
        lower: ScalarRefImpl<'_>,
        upper: ScalarRefImpl<'_>,
    ) -> Result<O, ScalarError> {
        let value = O::from_erased(value);
        let lower = O::from_erased(lower);
        let upper = O::from_erased(upper);
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

pub(crate) struct StringBinary {
    operator: StringOperator,
}

pub(crate) enum StringOperator {
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

pub(crate) struct BoolCompare(ComparisonOperator);

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

macro_rules! numeric_shell_enum {
    ($name:ident, $function:ident) => {
        pub(crate) enum $name {
            Int16(CheckedBinaryExpression<$function<i16>>),
            Int32(CheckedBinaryExpression<$function<i32>>),
            Int64(CheckedBinaryExpression<$function<i64>>),
            Float32(CheckedBinaryExpression<$function<f32>>),
            Float64(CheckedBinaryExpression<$function<f64>>),
        }

        impl $name {
            pub(crate) fn evaluate(
                &self,
                inputs: &[ColumnViewImpl<'_>],
            ) -> Result<ArrayImpl, ExpressionError> {
                match self {
                    Self::Int16(expression) => expression.evaluate(inputs),
                    Self::Int32(expression) => expression.evaluate(inputs),
                    Self::Int64(expression) => expression.evaluate(inputs),
                    Self::Float32(expression) => expression.evaluate(inputs),
                    Self::Float64(expression) => expression.evaluate(inputs),
                }
            }
        }

        impl Expression for $name {
            fn name(&self) -> &'static str {
                match self {
                    Self::Int16(expression) => expression.name(),
                    Self::Int32(expression) => expression.name(),
                    Self::Int64(expression) => expression.name(),
                    Self::Float32(expression) => expression.name(),
                    Self::Float64(expression) => expression.name(),
                }
            }

            fn input_types(&self) -> &[PhysicalType] {
                match self {
                    Self::Int16(expression) => expression.input_types(),
                    Self::Int32(expression) => expression.input_types(),
                    Self::Int64(expression) => expression.input_types(),
                    Self::Float32(expression) => expression.input_types(),
                    Self::Float64(expression) => expression.input_types(),
                }
            }

            fn output_type(&self) -> PhysicalType {
                match self {
                    Self::Int16(expression) => expression.output_type(),
                    Self::Int32(expression) => expression.output_type(),
                    Self::Int64(expression) => expression.output_type(),
                    Self::Float32(expression) => expression.output_type(),
                    Self::Float64(expression) => expression.output_type(),
                }
            }

            fn evaluate(
                &self,
                inputs: &[ColumnViewImpl<'_>],
            ) -> Result<ArrayImpl, ExpressionError> {
                self.evaluate(inputs)
            }
        }
    };
}

numeric_shell_enum!(NumericBinaryExpression, NumericBinary);
numeric_shell_enum!(NumericComparisonExpression, NumericCompare);

pub(crate) enum NumericNegExpression {
    Int16(UnaryExpression<NumericNeg<i16>>),
    Int32(UnaryExpression<NumericNeg<i32>>),
    Int64(UnaryExpression<NumericNeg<i64>>),
    Float32(UnaryExpression<NumericNeg<f32>>),
    Float64(UnaryExpression<NumericNeg<f64>>),
}

impl NumericNegExpression {
    pub(crate) fn evaluate(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<ArrayImpl, ExpressionError> {
        match self {
            Self::Int16(expression) => expression.evaluate(inputs),
            Self::Int32(expression) => expression.evaluate(inputs),
            Self::Int64(expression) => expression.evaluate(inputs),
            Self::Float32(expression) => expression.evaluate(inputs),
            Self::Float64(expression) => expression.evaluate(inputs),
        }
    }
}

impl Expression for NumericNegExpression {
    fn name(&self) -> &'static str {
        match self {
            Self::Int16(expression) => expression.name(),
            Self::Int32(expression) => expression.name(),
            Self::Int64(expression) => expression.name(),
            Self::Float32(expression) => expression.name(),
            Self::Float64(expression) => expression.name(),
        }
    }

    fn input_types(&self) -> &[PhysicalType] {
        match self {
            Self::Int16(expression) => expression.input_types(),
            Self::Int32(expression) => expression.input_types(),
            Self::Int64(expression) => expression.input_types(),
            Self::Float32(expression) => expression.input_types(),
            Self::Float64(expression) => expression.input_types(),
        }
    }

    fn output_type(&self) -> PhysicalType {
        match self {
            Self::Int16(expression) => expression.output_type(),
            Self::Int32(expression) => expression.output_type(),
            Self::Int64(expression) => expression.output_type(),
            Self::Float32(expression) => expression.output_type(),
            Self::Float64(expression) => expression.output_type(),
        }
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        self.evaluate(inputs)
    }
}

pub(crate) enum NumericClampExpression {
    Int16(TernaryExpression<NumericClamp<i16>>),
    Int32(TernaryExpression<NumericClamp<i32>>),
    Int64(TernaryExpression<NumericClamp<i64>>),
    Float32(TernaryExpression<NumericClamp<f32>>),
    Float64(TernaryExpression<NumericClamp<f64>>),
}

impl NumericClampExpression {
    pub(crate) fn evaluate(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<ArrayImpl, ExpressionError> {
        match self {
            Self::Int16(expression) => expression.evaluate(inputs),
            Self::Int32(expression) => expression.evaluate(inputs),
            Self::Int64(expression) => expression.evaluate(inputs),
            Self::Float32(expression) => expression.evaluate(inputs),
            Self::Float64(expression) => expression.evaluate(inputs),
        }
    }
}

impl Expression for NumericClampExpression {
    fn name(&self) -> &'static str {
        match self {
            Self::Int16(expression) => expression.name(),
            Self::Int32(expression) => expression.name(),
            Self::Int64(expression) => expression.name(),
            Self::Float32(expression) => expression.name(),
            Self::Float64(expression) => expression.name(),
        }
    }

    fn input_types(&self) -> &[PhysicalType] {
        match self {
            Self::Int16(expression) => expression.input_types(),
            Self::Int32(expression) => expression.input_types(),
            Self::Int64(expression) => expression.input_types(),
            Self::Float32(expression) => expression.input_types(),
            Self::Float64(expression) => expression.input_types(),
        }
    }

    fn output_type(&self) -> PhysicalType {
        match self {
            Self::Int16(expression) => expression.output_type(),
            Self::Int32(expression) => expression.output_type(),
            Self::Int64(expression) => expression.output_type(),
            Self::Float32(expression) => expression.output_type(),
            Self::Float64(expression) => expression.output_type(),
        }
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        self.evaluate(inputs)
    }
}

macro_rules! build_numeric_shell {
    ($output:expr, $enum:ident, $name:expr, $inputs:expr, $function:expr) => {
        match $output {
            PhysicalType::Int16 => $enum::Int16(CheckedBinaryExpression::new(
                $name,
                $inputs,
                $function(PhantomData::<i16>),
            )),
            PhysicalType::Int32 => $enum::Int32(CheckedBinaryExpression::new(
                $name,
                $inputs,
                $function(PhantomData::<i32>),
            )),
            PhysicalType::Int64 => $enum::Int64(CheckedBinaryExpression::new(
                $name,
                $inputs,
                $function(PhantomData::<i64>),
            )),
            PhysicalType::Float32 => $enum::Float32(CheckedBinaryExpression::new(
                $name,
                $inputs,
                $function(PhantomData::<f32>),
            )),
            PhysicalType::Float64 => $enum::Float64(CheckedBinaryExpression::new(
                $name,
                $inputs,
                $function(PhantomData::<f64>),
            )),
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
) -> NumericBinaryExpression {
    let inputs = [left, right];
    build_numeric_shell!(output, NumericBinaryExpression, name, inputs, |marker| {
        NumericBinary { operator, marker }
    })
}

pub(crate) fn build_numeric_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
    left: PhysicalType,
    right: PhysicalType,
    common: PhysicalType,
) -> NumericComparisonExpression {
    let inputs = [left, right];
    build_numeric_shell!(
        common,
        NumericComparisonExpression,
        name,
        inputs,
        |marker| { NumericCompare { operator, marker } }
    )
}

pub(crate) fn build_numeric_neg_expression(
    name: &'static str,
    input: PhysicalType,
) -> NumericNegExpression {
    match input {
        PhysicalType::Int16 => NumericNegExpression::Int16(UnaryExpression::new(
            name,
            [PhysicalType::Int16],
            NumericNeg(PhantomData),
        )),
        PhysicalType::Int32 => NumericNegExpression::Int32(UnaryExpression::new(
            name,
            [PhysicalType::Int32],
            NumericNeg(PhantomData),
        )),
        PhysicalType::Int64 => NumericNegExpression::Int64(UnaryExpression::new(
            name,
            [PhysicalType::Int64],
            NumericNeg(PhantomData),
        )),
        PhysicalType::Float32 => NumericNegExpression::Float32(UnaryExpression::new(
            name,
            [PhysicalType::Float32],
            NumericNeg(PhantomData),
        )),
        PhysicalType::Float64 => NumericNegExpression::Float64(UnaryExpression::new(
            name,
            [PhysicalType::Float64],
            NumericNeg(PhantomData),
        )),
        _ => unreachable!("numeric input"),
    }
}

pub(crate) fn build_numeric_clamp_expression(
    name: &'static str,
    inputs: [PhysicalType; 3],
    output: PhysicalType,
) -> NumericClampExpression {
    match output {
        PhysicalType::Int16 => NumericClampExpression::Int16(TernaryExpression::new(
            name,
            inputs,
            NumericClamp(PhantomData),
        )),
        PhysicalType::Int32 => NumericClampExpression::Int32(TernaryExpression::new(
            name,
            inputs,
            NumericClamp(PhantomData),
        )),
        PhysicalType::Int64 => NumericClampExpression::Int64(TernaryExpression::new(
            name,
            inputs,
            NumericClamp(PhantomData),
        )),
        PhysicalType::Float32 => NumericClampExpression::Float32(TernaryExpression::new(
            name,
            inputs,
            NumericClamp(PhantomData),
        )),
        PhysicalType::Float64 => NumericClampExpression::Float64(TernaryExpression::new(
            name,
            inputs,
            NumericClamp(PhantomData),
        )),
        _ => unreachable!("numeric output"),
    }
}

pub(crate) fn build_string_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
) -> CheckedBinaryExpression<StringBinary> {
    CheckedBinaryExpression::new(
        name,
        [const { PhysicalType::String }; 2],
        StringBinary {
            operator: StringOperator::Compare(operator),
        },
    )
}

pub(crate) fn build_string_contains_expression(
    name: &'static str,
) -> CheckedBinaryExpression<StringBinary> {
    CheckedBinaryExpression::new(
        name,
        [const { PhysicalType::String }; 2],
        StringBinary {
            operator: StringOperator::Contains,
        },
    )
}

pub(crate) fn build_bool_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
) -> CheckedBinaryExpression<BoolCompare> {
    CheckedBinaryExpression::new(
        name,
        [const { PhysicalType::Bool }; 2],
        BoolCompare(operator),
    )
}
