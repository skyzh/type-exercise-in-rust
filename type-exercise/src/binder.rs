use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    ArithmeticOperator, ArrayImpl, AsyncExpression, BatchFuture, BooleanOperator, ColumnViewImpl,
    ComparisonOperator, DataType, Expression, ExpressionError, PhysicalType, PrimitiveLoop,
    build_bool_comparison_expression, build_boolean_expression, build_builtin_expression,
    build_numeric_binary_expression, build_numeric_clamp_expression,
    build_numeric_comparison_expression, build_numeric_neg_expression,
    build_string_comparison_expression, build_string_contains_expression, promote_numeric,
};

/// A checked failure while selecting one physical expression.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(not(feature = "opaque-errors"))]
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

/// The opaque learner-layout variant used only by the compile-based
/// compatibility fixture: no public variant can be named or matched, so any
/// copied test that references the real layout fails to compile against it.
#[cfg(feature = "opaque-errors")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindError {
    #[doc(hidden)]
    Hidden(String),
}

impl BindError {
    #[cfg(not(feature = "opaque-errors"))]
    pub(crate) fn unknown_function(name: String) -> Self {
        Self::UnknownFunction { name }
    }

    #[cfg(feature = "opaque-errors")]
    pub(crate) fn unknown_function(name: String) -> Self {
        Self::Hidden(format!("unknown function `{name}`"))
    }

    #[cfg(not(feature = "opaque-errors"))]
    pub(crate) fn arity_mismatch(name: String, expected: usize, actual: usize) -> Self {
        Self::InputArityMismatch {
            name,
            expected,
            actual,
        }
    }

    #[cfg(feature = "opaque-errors")]
    pub(crate) fn arity_mismatch(name: String, expected: usize, actual: usize) -> Self {
        Self::Hidden(format!(
            "function `{name}` expects {expected} arguments, got {actual}"
        ))
    }

    #[cfg(not(feature = "opaque-errors"))]
    pub(crate) fn unsupported(name: String, inputs: Vec<DataType>) -> Self {
        Self::UnsupportedArguments { name, inputs }
    }

    #[cfg(feature = "opaque-errors")]
    pub(crate) fn unsupported(name: String, inputs: Vec<DataType>) -> Self {
        Self::Hidden(format!("function `{name}` does not support {inputs:?}"))
    }

    #[cfg(not(feature = "opaque-errors"))]
    pub(crate) fn missing_physical(name: &'static str) -> Self {
        Self::MissingPhysicalExpression { name }
    }

    #[cfg(feature = "opaque-errors")]
    pub(crate) fn missing_physical(name: &'static str) -> Self {
        Self::Hidden(format!("physical expression `{name}` is not registered"))
    }

    #[cfg(not(feature = "opaque-errors"))]
    pub(crate) fn physical_mismatch(
        name: &'static str,
        expected_inputs: Vec<PhysicalType>,
        actual_inputs: Vec<PhysicalType>,
        expected_output: PhysicalType,
        actual_output: PhysicalType,
    ) -> Self {
        Self::PhysicalSignatureMismatch {
            name,
            expected_inputs,
            actual_inputs,
            expected_output,
            actual_output,
        }
    }

    #[cfg(feature = "opaque-errors")]
    pub(crate) fn physical_mismatch(
        name: &'static str,
        expected_inputs: Vec<PhysicalType>,
        actual_inputs: Vec<PhysicalType>,
        expected_output: PhysicalType,
        actual_output: PhysicalType,
    ) -> Self {
        Self::Hidden(format!(
            "physical signature mismatch for `{name}`: inputs {expected_inputs:?} -> {actual_inputs:?}, output {expected_output:?} -> {actual_output:?}"
        ))
    }
}

#[cfg(not(feature = "opaque-errors"))]
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

#[cfg(not(feature = "opaque-errors"))]
impl Error for BindError {}

#[cfg(feature = "opaque-errors")]
impl Display for BindError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hidden(message) => formatter.write_str(message),
        }
    }
}

#[cfg(feature = "opaque-errors")]
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
            return Err(BindError::physical_mismatch(
                expression.name(),
                expected_inputs,
                actual_inputs,
                expected_output,
                actual_output,
            ));
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
                return Err(BindError::arity_mismatch(
                    error_name.clone(),
                    2,
                    inputs.len(),
                ));
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
                return Err(BindError::arity_mismatch(
                    error_name.clone(),
                    1,
                    inputs.len(),
                ));
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
                return Err(BindError::arity_mismatch(
                    error_name.clone(),
                    3,
                    inputs.len(),
                ));
            };
            factory(first.clone(), second.clone(), third.clone())
        });
    }

    pub fn bind(&self, name: &str, inputs: &[DataType]) -> Result<BoundExpression, BindError> {
        self.functions
            .get(name)
            .ok_or_else(|| BindError::unknown_function(name.to_owned()))?(inputs)
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
    build_builtin_expression(name).ok_or_else(|| BindError::missing_physical(name))
}

fn unsupported(name: &str, inputs: impl IntoIterator<Item = DataType>) -> BindError {
    BindError::unsupported(name.to_owned(), inputs.into_iter().collect())
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
        return Err(BindError::unsupported(
            "concat".to_owned(),
            vec![left, right],
        ));
    }
    BoundExpression::new(
        build_physical("string_concat")?,
        [left, right],
        DataType::Varchar,
    )
}

#[cfg(test)]
mod tests {
    use super::{BindError, FunctionRegistry};
    use crate::DataType;

    #[test]
    fn decimal_storage_does_not_enable_operators_or_cast_like_coercion() {
        let decimal = DataType::decimal(8, 2).unwrap();
        let registry = FunctionRegistry::with_builtins();

        for (name, inputs) in [
            ("+", vec![decimal.clone(), decimal.clone()]),
            ("-", vec![decimal.clone(), DataType::Integer]),
            ("*", vec![DataType::Integer, decimal.clone()]),
            ("/", vec![decimal.clone(), decimal.clone()]),
            ("neg", vec![decimal.clone()]),
            (
                "clamp",
                vec![decimal.clone(), decimal.clone(), decimal.clone()],
            ),
            ("<", vec![decimal.clone(), decimal.clone()]),
            ("<=", vec![decimal.clone(), decimal.clone()]),
            (">", vec![decimal.clone(), decimal.clone()]),
            (">=", vec![decimal.clone(), decimal.clone()]),
            ("=", vec![decimal.clone(), decimal.clone()]),
            ("!=", vec![decimal.clone(), decimal.clone()]),
            ("contains", vec![decimal.clone(), DataType::Varchar]),
            ("concat", vec![DataType::Varchar, decimal.clone()]),
        ] {
            #[cfg(not(feature = "opaque-errors"))]
            assert!(matches!(
                registry.bind(name, &inputs),
                Err(BindError::UnsupportedArguments { .. })
            ));
            #[cfg(feature = "opaque-errors")]
            assert!(registry.bind(name, &inputs).is_err());
        }
    }
}
