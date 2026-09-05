use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    ArrayImpl, ColumnViewImpl, DataType, Expression, PHYSICAL_FUNCTION_CATALOG, PhysicalFunction,
    PhysicalType, build_physical_expression,
};

/// One logical function call before a physical expression has been selected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalCall {
    name: String,
    input_types: Box<[DataType]>,
}

impl LogicalCall {
    pub fn new(name: impl Into<String>, input_types: impl IntoIterator<Item = DataType>) -> Self {
        Self {
            name: name.into(),
            input_types: input_types.into_iter().collect(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn input_types(&self) -> &[DataType] {
        &self.input_types
    }
}

/// A checked failure while binding a logical call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindError {
    UnknownFunction {
        name: String,
    },
    InputArityMismatch {
        name: String,
        expected: usize,
        actual: usize,
    },
    UnsupportedArguments {
        name: String,
        inputs: Vec<DataType>,
    },
    PhysicalSignatureMismatch {
        expected_inputs: Vec<PhysicalType>,
        actual_inputs: Vec<PhysicalType>,
        expected_output: PhysicalType,
        actual_output: PhysicalType,
    },
}

impl Display for BindError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownFunction { name } => write!(formatter, "unknown function `{name}`"),
            Self::InputArityMismatch {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "function `{name}` expects {expected} arguments, got {actual}"
            ),
            Self::UnsupportedArguments { name, inputs } => {
                write!(formatter, "function `{name}` does not support {inputs:?}")
            }
            Self::PhysicalSignatureMismatch {
                expected_inputs,
                actual_inputs,
                expected_output,
                actual_output,
            } => write!(
                formatter,
                "physical signature mismatch: expected {expected_inputs:?} -> {expected_output:?}, got {actual_inputs:?} -> {actual_output:?}"
            ),
        }
    }
}

impl Error for BindError {}

/// One logical call paired with its selected whole-batch physical expression.
pub struct BoundExpression {
    call: LogicalCall,
    output_type: DataType,
    expression: Box<dyn Expression>,
}

impl BoundExpression {
    pub fn new(
        call: LogicalCall,
        output_type: DataType,
        expression: Box<dyn Expression>,
    ) -> Result<Self, BindError> {
        let expected_inputs = call
            .input_types()
            .iter()
            .map(DataType::physical_type)
            .collect::<Vec<_>>();
        let actual_inputs = expression.input_types().to_vec();
        let expected_output = output_type.physical_type();
        let actual_output = expression.output_type();
        if expected_inputs != actual_inputs || expected_output != actual_output {
            return Err(BindError::PhysicalSignatureMismatch {
                expected_inputs,
                actual_inputs,
                expected_output,
                actual_output,
            });
        }

        Ok(Self {
            call,
            output_type,
            expression,
        })
    }

    pub fn logical_call(&self) -> &LogicalCall {
        &self.call
    }

    pub fn output_type(&self) -> &DataType {
        &self.output_type
    }

    pub fn physical_expression(&self) -> &dyn Expression {
        self.expression.as_ref()
    }

    pub fn into_physical_expression(self) -> Box<dyn Expression> {
        self.expression
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.expression.evaluate(inputs)
    }
}

fn logical_function(name: &str) -> Option<PhysicalFunction> {
    Some(match name {
        "+" => PhysicalFunction::Add,
        "-" => PhysicalFunction::Subtract,
        "*" => PhysicalFunction::Multiply,
        "/" => PhysicalFunction::Divide,
        "neg" => PhysicalFunction::Negate,
        "clamp" => PhysicalFunction::Clamp,
        "<" => PhysicalFunction::Less,
        "<=" => PhysicalFunction::LessOrEqual,
        ">" => PhysicalFunction::Greater,
        ">=" => PhysicalFunction::GreaterOrEqual,
        "=" => PhysicalFunction::Equal,
        "!=" => PhysicalFunction::NotEqual,
        "boolean_and" => PhysicalFunction::BooleanAnd,
        "boolean_or" => PhysicalFunction::BooleanOr,
        "boolean_not" => PhysicalFunction::BooleanNot,
        "concat" => PhysicalFunction::StringConcat,
        "contains" => PhysicalFunction::StringContains,
        _ => return None,
    })
}

fn expected_arity(function: PhysicalFunction) -> usize {
    PHYSICAL_FUNCTION_CATALOG
        .iter()
        .find(|entry| entry.function == function)
        .expect("every logical function maps to catalog metadata")
        .arity
}

fn unsupported(call: &LogicalCall) -> BindError {
    BindError::UnsupportedArguments {
        name: call.name().to_owned(),
        inputs: call.input_types().to_vec(),
    }
}

fn promote_numeric(left: &DataType, right: &DataType) -> Option<DataType> {
    use DataType::*;
    Some(match (left, right) {
        (SmallInt, SmallInt) => SmallInt,
        (SmallInt, Integer) | (Integer, SmallInt) | (Integer, Integer) => Integer,
        (SmallInt, BigInt)
        | (BigInt, SmallInt)
        | (Integer, BigInt)
        | (BigInt, Integer)
        | (BigInt, BigInt) => BigInt,
        (SmallInt, Real) | (Real, SmallInt) | (Real, Real) => Real,
        (SmallInt, Double)
        | (Double, SmallInt)
        | (Integer, Real)
        | (Real, Integer)
        | (Integer, Double)
        | (Double, Integer)
        | (Real, Double)
        | (Double, Real)
        | (Double, Double) => Double,
        _ => return None,
    })
}

fn output_type(call: &LogicalCall, function: PhysicalFunction) -> Result<DataType, BindError> {
    let inputs = call.input_types();
    match function {
        PhysicalFunction::Add
        | PhysicalFunction::Subtract
        | PhysicalFunction::Multiply
        | PhysicalFunction::Divide => {
            let [left, right] = inputs else {
                unreachable!("arity is checked before logical overload resolution")
            };
            promote_numeric(left, right).ok_or_else(|| unsupported(call))
        }
        PhysicalFunction::Negate => {
            let [input] = inputs else {
                unreachable!("arity is checked before logical overload resolution")
            };
            promote_numeric(input, input).ok_or_else(|| unsupported(call))
        }
        PhysicalFunction::Clamp => {
            let [value, lower, upper] = inputs else {
                unreachable!("arity is checked before logical overload resolution")
            };
            promote_numeric(value, lower)
                .and_then(|pair| promote_numeric(&pair, upper))
                .ok_or_else(|| unsupported(call))
        }
        PhysicalFunction::Less
        | PhysicalFunction::LessOrEqual
        | PhysicalFunction::Greater
        | PhysicalFunction::GreaterOrEqual => {
            let [left, right] = inputs else {
                unreachable!("arity is checked before logical overload resolution")
            };
            (promote_numeric(left, right).is_some() || left.is_string() && right.is_string())
                .then_some(DataType::Boolean)
                .ok_or_else(|| unsupported(call))
        }
        PhysicalFunction::Equal | PhysicalFunction::NotEqual => {
            let [left, right] = inputs else {
                unreachable!("arity is checked before logical overload resolution")
            };
            (promote_numeric(left, right).is_some()
                || left.is_string() && right.is_string()
                || *left == DataType::Boolean && *right == DataType::Boolean)
                .then_some(DataType::Boolean)
                .ok_or_else(|| unsupported(call))
        }
        PhysicalFunction::BooleanAnd | PhysicalFunction::BooleanOr => (inputs
            == [DataType::Boolean, DataType::Boolean])
        .then_some(DataType::Boolean)
        .ok_or_else(|| unsupported(call)),
        PhysicalFunction::BooleanNot => (inputs == [DataType::Boolean])
            .then_some(DataType::Boolean)
            .ok_or_else(|| unsupported(call)),
        PhysicalFunction::StringConcat => {
            let [left, right] = inputs else {
                unreachable!("arity is checked before logical overload resolution")
            };
            (left.is_string() && right.is_string())
                .then_some(DataType::Varchar)
                .ok_or_else(|| unsupported(call))
        }
        PhysicalFunction::StringContains => {
            let [left, right] = inputs else {
                unreachable!("arity is checked before logical overload resolution")
            };
            (left.is_string() && right.is_string())
                .then_some(DataType::Boolean)
                .ok_or_else(|| unsupported(call))
        }
    }
}

/// Resolve one logical function call to a unique physical catalog entry.
pub fn bind_logical_call(call: LogicalCall) -> Result<BoundExpression, BindError> {
    let function = logical_function(call.name()).ok_or_else(|| BindError::UnknownFunction {
        name: call.name().to_owned(),
    })?;
    let expected = expected_arity(function);
    if call.input_types().len() != expected {
        return Err(BindError::InputArityMismatch {
            name: call.name().to_owned(),
            expected,
            actual: call.input_types().len(),
        });
    }

    let output_type = output_type(&call, function)?;
    let physical_inputs = call
        .input_types()
        .iter()
        .map(DataType::physical_type)
        .collect::<Vec<_>>();
    let expression =
        build_physical_expression(function, &physical_inputs).map_err(|_| unsupported(&call))?;
    BoundExpression::new(call, output_type, expression)
}
