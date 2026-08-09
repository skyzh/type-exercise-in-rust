use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    ArrayImpl, AsyncExpression, BatchFuture, ColumnViewImpl, DataType, Expression, ExpressionError,
    PhysicalType, PrimitiveLoop, build_builtin_expression,
};

/// A checked failure while selecting one physical expression.
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
    MissingPhysicalExpression {
        name: &'static str,
    },
    PhysicalSignatureMismatch {
        name: &'static str,
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
            Self::MissingPhysicalExpression { name } => {
                write!(formatter, "physical expression `{name}` is not registered")
            }
            Self::PhysicalSignatureMismatch {
                name,
                expected_inputs,
                actual_inputs,
                expected_output,
                actual_output,
            } => write!(
                formatter,
                "physical signature mismatch for `{name}`: expected {expected_inputs:?} -> {expected_output:?}, got {actual_inputs:?} -> {actual_output:?}"
            ),
        }
    }
}

impl Error for BindError {}

/// One runtime expression paired with its checked logical signature.
pub struct BoundExpression {
    expression: Box<dyn Expression>,
    input_types: Box<[DataType]>,
    output_type: DataType,
}

impl BoundExpression {
    pub fn new(
        expression: Box<dyn Expression>,
        input_types: impl IntoIterator<Item = DataType>,
        output_type: DataType,
    ) -> Result<Self, BindError> {
        let input_types = input_types.into_iter().collect::<Box<[_]>>();
        let expected_inputs = input_types
            .iter()
            .copied()
            .map(DataType::physical_type)
            .collect::<Vec<_>>();
        let actual_inputs = expression.input_types().to_vec();
        let expected_output = output_type.physical_type();
        let actual_output = expression.output_type();
        if actual_inputs != expected_inputs || actual_output != expected_output {
            return Err(BindError::PhysicalSignatureMismatch {
                name: expression.name(),
                expected_inputs,
                actual_inputs,
                expected_output,
                actual_output,
            });
        }

        Ok(Self {
            expression,
            input_types,
            output_type,
        })
    }

    pub fn input_types(&self) -> &[DataType] {
        &self.input_types
    }

    pub fn output_type(&self) -> DataType {
        self.output_type
    }

    pub fn physical_name(&self) -> &'static str {
        self.expression.name()
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        self.expression.evaluate(inputs)
    }

    pub fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<(ArrayImpl, PrimitiveLoop), ExpressionError> {
        self.expression.evaluate_with_loop(inputs)
    }
}

impl AsyncExpression for BoundExpression {
    fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a> {
        Box::pin(async move { self.evaluate(inputs) })
    }
}

type FunctionFactory =
    dyn Fn(&[DataType]) -> Result<BoundExpression, BindError> + Send + Sync + 'static;

/// Planning-time registry for logical functions of any arity.
#[derive(Default)]
pub struct FunctionRegistry {
    functions: HashMap<String, Box<FunctionFactory>>,
}

impl FunctionRegistry {
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        registry.register_binary("+", bind_add);
        registry.register_binary("concat", bind_concat);
        registry
    }

    pub fn register(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(&[DataType]) -> Result<BoundExpression, BindError> + Send + Sync + 'static,
    ) {
        self.functions.insert(name.into(), Box::new(factory));
    }

    /// Retain the original binary registration surface while storing a slice factory.
    pub fn register_binary(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(DataType, DataType) -> Result<BoundExpression, BindError>
        + Send
        + Sync
        + 'static,
    ) {
        let name = name.into();
        let error_name = name.clone();
        self.register(name, move |inputs| {
            let [left, right] = inputs else {
                return Err(BindError::InputArityMismatch {
                    name: error_name.clone(),
                    expected: 2,
                    actual: inputs.len(),
                });
            };
            factory(*left, *right)
        });
    }

    pub fn bind(&self, name: &str, inputs: &[DataType]) -> Result<BoundExpression, BindError> {
        self.functions
            .get(name)
            .ok_or_else(|| BindError::UnknownFunction {
                name: name.to_owned(),
            })?(inputs)
    }

    /// Retain the original binary binding surface on top of the slice registry.
    pub fn bind_binary(
        &self,
        name: &str,
        left: DataType,
        right: DataType,
    ) -> Result<BoundExpression, BindError> {
        self.bind(name, &[left, right])
    }
}

fn build_physical(name: &'static str) -> Result<Box<dyn Expression>, BindError> {
    build_builtin_expression(name).ok_or(BindError::MissingPhysicalExpression { name })
}

fn bind_add(left: DataType, right: DataType) -> Result<BoundExpression, BindError> {
    if (left, right) != (DataType::Integer, DataType::Integer) {
        return Err(BindError::UnsupportedArguments {
            name: "+".to_owned(),
            inputs: vec![left, right],
        });
    }
    BoundExpression::new(build_physical("i32_add")?, [left, right], DataType::Integer)
}

fn bind_concat(left: DataType, right: DataType) -> Result<BoundExpression, BindError> {
    if !left.is_string() || !right.is_string() {
        return Err(BindError::UnsupportedArguments {
            name: "concat".to_owned(),
            inputs: vec![left, right],
        });
    }
    BoundExpression::new(
        build_physical("string_concat")?,
        [left, right],
        DataType::Varchar,
    )
}
