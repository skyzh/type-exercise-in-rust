use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    ArithmeticOperator, ArrayImpl, BooleanOperator, ColumnViewImpl, ComparisonOperator, DataType,
    Expression, I32Add, Nullability, PhysicalType, PrimitiveBinaryExpression, PrimitiveLoop,
    build_bool_comparison_expression, build_boolean_expression, build_numeric_binary_expression,
    build_numeric_clamp_expression, build_numeric_comparison_expression,
    build_numeric_neg_expression, build_string_comparison_expression,
    build_string_contains_expression, promote_numeric,
};

/// The two physical expressions available before logical binding is introduced.
pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &["i32_add", "string_concat"];

pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
    match name {
        "i32_add" => Some(Box::new(PrimitiveBinaryExpression::new("i32_add", I32Add))),
        "string_concat" => Some(Box::new(crate::string::build_string_concat_expression())),
        _ => None,
    }
}

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
        self.output_type.clone()
    }

    pub fn physical_name(&self) -> &'static str {
        self.expression.name()
    }

    pub fn output_nullability(&self, inputs: &[Nullability]) -> Nullability {
        self.expression.output_nullability(inputs)
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.expression.evaluate(inputs)
    }

    pub fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)> {
        self.expression.evaluate_with_loop(inputs)
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
        for (name, operator) in [
            ("+", ArithmeticOperator::Add),
            ("-", ArithmeticOperator::Subtract),
            ("*", ArithmeticOperator::Multiply),
            ("/", ArithmeticOperator::Divide),
        ] {
            registry.register_binary(name, move |left, right| {
                bind_arithmetic(name, operator, left, right)
            });
        }
        registry.register_unary("neg", bind_neg);
        registry.register_ternary("clamp", bind_clamp);
        for (name, operator) in [
            ("<", ComparisonOperator::Less),
            ("<=", ComparisonOperator::LessOrEqual),
            (">", ComparisonOperator::Greater),
            (">=", ComparisonOperator::GreaterOrEqual),
            ("=", ComparisonOperator::Equal),
            ("!=", ComparisonOperator::NotEqual),
        ] {
            registry.register_binary(name, move |left, right| {
                bind_comparison(name, operator, left, right)
            });
        }
        registry.register_binary("contains", bind_contains);
        registry.register_binary("concat", bind_concat);
        registry.register_binary("boolean_and", |left, right| {
            bind_boolean("boolean_and", BooleanOperator::And, left, Some(right))
        });
        registry.register_binary("boolean_or", |left, right| {
            bind_boolean("boolean_or", BooleanOperator::Or, left, Some(right))
        });
        registry.register_unary("boolean_not", |input| {
            bind_boolean("boolean_not", BooleanOperator::Not, input, None)
        });
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
            factory(left.clone(), right.clone())
        });
    }

    pub fn register_unary(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(DataType) -> Result<BoundExpression, BindError> + Send + Sync + 'static,
    ) {
        let name = name.into();
        let error_name = name.clone();
        self.register(name, move |inputs| {
            let [input] = inputs else {
                return Err(BindError::InputArityMismatch {
                    name: error_name.clone(),
                    expected: 1,
                    actual: inputs.len(),
                });
            };
            factory(input.clone())
        });
    }

    pub fn register_ternary(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(DataType, DataType, DataType) -> Result<BoundExpression, BindError>
        + Send
        + Sync
        + 'static,
    ) {
        let name = name.into();
        let error_name = name.clone();
        self.register(name, move |inputs| {
            let [first, second, third] = inputs else {
                return Err(BindError::InputArityMismatch {
                    name: error_name.clone(),
                    expected: 3,
                    actual: inputs.len(),
                });
            };
            factory(first.clone(), second.clone(), third.clone())
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

fn unsupported(name: &str, inputs: impl IntoIterator<Item = DataType>) -> BindError {
    BindError::UnsupportedArguments {
        name: name.to_owned(),
        inputs: inputs.into_iter().collect(),
    }
}

fn bind_arithmetic(
    name: &'static str,
    operator: ArithmeticOperator,
    left: DataType,
    right: DataType,
) -> Result<BoundExpression, BindError> {
    let output = promote_numeric(&left, &right)
        .ok_or_else(|| unsupported(name, [left.clone(), right.clone()]))?;
    let expression = if operator == ArithmeticOperator::Add
        && left == DataType::Integer
        && right == DataType::Integer
    {
        build_physical("i32_add")?
    } else {
        Box::new(build_numeric_binary_expression(
            match operator {
                ArithmeticOperator::Add => "numeric_add",
                ArithmeticOperator::Subtract => "numeric_subtract",
                ArithmeticOperator::Multiply => "numeric_multiply",
                ArithmeticOperator::Divide => "numeric_divide",
            },
            operator,
            left.physical_type(),
            right.physical_type(),
            output.physical_type(),
        ))
    };
    BoundExpression::new(expression, [left, right], output)
}

fn bind_neg(input: DataType) -> Result<BoundExpression, BindError> {
    promote_numeric(&input, &input).ok_or_else(|| unsupported("neg", [input.clone()]))?;
    BoundExpression::new(
        Box::new(build_numeric_neg_expression(
            "numeric_neg",
            input.physical_type(),
        )),
        [input.clone()],
        input,
    )
}

fn bind_clamp(
    value: DataType,
    lower: DataType,
    upper: DataType,
) -> Result<BoundExpression, BindError> {
    let pair = promote_numeric(&value, &lower)
        .ok_or_else(|| unsupported("clamp", [value.clone(), lower.clone(), upper.clone()]))?;
    let output = promote_numeric(&pair, &upper)
        .ok_or_else(|| unsupported("clamp", [value.clone(), lower.clone(), upper.clone()]))?;
    BoundExpression::new(
        Box::new(build_numeric_clamp_expression(
            "numeric_clamp",
            [
                value.physical_type(),
                lower.physical_type(),
                upper.physical_type(),
            ],
            output.physical_type(),
        )),
        [value, lower, upper],
        output,
    )
}

fn bind_comparison(
    name: &'static str,
    operator: ComparisonOperator,
    left: DataType,
    right: DataType,
) -> Result<BoundExpression, BindError> {
    let expression: Box<dyn Expression> = if let Some(common) = promote_numeric(&left, &right) {
        Box::new(build_numeric_comparison_expression(
            match operator {
                ComparisonOperator::Less => "numeric_less",
                ComparisonOperator::LessOrEqual => "numeric_less_or_equal",
                ComparisonOperator::Greater => "numeric_greater",
                ComparisonOperator::GreaterOrEqual => "numeric_greater_or_equal",
                ComparisonOperator::Equal => "numeric_equal",
                ComparisonOperator::NotEqual => "numeric_not_equal",
            },
            operator,
            left.physical_type(),
            right.physical_type(),
            common.physical_type(),
        ))
    } else if left.is_string() && right.is_string() {
        Box::new(build_string_comparison_expression(
            match operator {
                ComparisonOperator::Less => "string_less",
                ComparisonOperator::LessOrEqual => "string_less_or_equal",
                ComparisonOperator::Greater => "string_greater",
                ComparisonOperator::GreaterOrEqual => "string_greater_or_equal",
                ComparisonOperator::Equal => "string_equal",
                ComparisonOperator::NotEqual => "string_not_equal",
            },
            operator,
        ))
    } else if matches!(
        operator,
        ComparisonOperator::Equal | ComparisonOperator::NotEqual
    ) && left == DataType::Boolean
        && right == DataType::Boolean
    {
        Box::new(build_bool_comparison_expression(
            match operator {
                ComparisonOperator::Equal => "bool_equal",
                ComparisonOperator::NotEqual => "bool_not_equal",
                _ => unreachable!("ordered boolean comparison is rejected"),
            },
            operator,
        ))
    } else {
        return Err(unsupported(name, [left, right]));
    };
    BoundExpression::new(expression, [left, right], DataType::Boolean)
}

fn bind_boolean(
    name: &'static str,
    operator: BooleanOperator,
    left: DataType,
    right: Option<DataType>,
) -> Result<BoundExpression, BindError> {
    let inputs = match right {
        Some(right) => vec![left, right],
        None => vec![left],
    };
    if !inputs.iter().all(|input| *input == DataType::Boolean) {
        return Err(unsupported(name, inputs));
    }
    BoundExpression::new(
        Box::new(build_boolean_expression(operator)),
        inputs,
        DataType::Boolean,
    )
}

fn bind_contains(left: DataType, right: DataType) -> Result<BoundExpression, BindError> {
    if !left.is_string() || !right.is_string() {
        return Err(unsupported("contains", [left, right]));
    }
    BoundExpression::new(
        Box::new(build_string_contains_expression("string_contains")),
        [left, right],
        DataType::Boolean,
    )
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
