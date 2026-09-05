use crate::{
    ArithmeticOperator, BooleanOperator, ComparisonOperator, Expression, PhysicalType,
    boolean::{
        build_bool_comparison_expression, build_boolean_binary_expression,
        build_boolean_not_expression,
    },
    numeric::{
        build_numeric_binary_expression, build_numeric_clamp_expression,
        build_numeric_comparison_expression, build_numeric_neg_expression,
    },
    string::{
        build_string_comparison_expression, build_string_concat_expression,
        build_string_contains_expression,
    },
};

/// The concrete physical functions available before logical binding exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalFunction {
    Add,
    Subtract,
    Multiply,
    Divide,
    Negate,
    Clamp,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
    BooleanAnd,
    BooleanOr,
    BooleanNot,
    StringConcat,
    StringContains,
}

/// Discoverable metadata for one physical function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalFunctionEntry {
    pub function: PhysicalFunction,
    pub name: &'static str,
    pub arity: usize,
}

/// The complete function-level physical catalog.
pub const PHYSICAL_FUNCTION_CATALOG: &[PhysicalFunctionEntry] = &[
    PhysicalFunctionEntry {
        function: PhysicalFunction::Add,
        name: "numeric_add",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::Subtract,
        name: "numeric_subtract",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::Multiply,
        name: "numeric_multiply",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::Divide,
        name: "numeric_divide",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::Negate,
        name: "numeric_negate",
        arity: 1,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::Clamp,
        name: "numeric_clamp",
        arity: 3,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::Less,
        name: "less",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::LessOrEqual,
        name: "less_or_equal",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::Greater,
        name: "greater",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::GreaterOrEqual,
        name: "greater_or_equal",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::Equal,
        name: "equal",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::NotEqual,
        name: "not_equal",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::BooleanAnd,
        name: "boolean_and",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::BooleanOr,
        name: "boolean_or",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::BooleanNot,
        name: "boolean_not",
        arity: 1,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::StringConcat,
        name: "string_concat",
        arity: 2,
    },
    PhysicalFunctionEntry {
        function: PhysicalFunction::StringContains,
        name: "string_contains",
        arity: 2,
    },
];

/// Look up a physical function identifier by its catalog name.
pub fn find_physical_function(name: &str) -> Option<PhysicalFunction> {
    PHYSICAL_FUNCTION_CATALOG
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.function)
}

fn entry(function: PhysicalFunction) -> &'static PhysicalFunctionEntry {
    PHYSICAL_FUNCTION_CATALOG
        .iter()
        .find(|entry| entry.function == function)
        .expect("every public physical function has catalog metadata")
}

fn numeric_output(left: &PhysicalType, right: &PhysicalType) -> Option<PhysicalType> {
    use PhysicalType::*;
    Some(match (left, right) {
        (Int16, Int16) => Int16,
        (Int16, Int32) | (Int32, Int16) | (Int32, Int32) => Int32,
        (Int16, Int64) | (Int64, Int16) | (Int32, Int64) | (Int64, Int32) | (Int64, Int64) => Int64,
        (Int16, Float32) | (Float32, Int16) | (Float32, Float32) => Float32,
        (Int16, Float64)
        | (Float64, Int16)
        | (Int32, Float32)
        | (Float32, Int32)
        | (Int32, Float64)
        | (Float64, Int32)
        | (Float32, Float64)
        | (Float64, Float32)
        | (Float64, Float64) => Float64,
        _ => return None,
    })
}

fn arithmetic_operator(function: PhysicalFunction) -> Option<ArithmeticOperator> {
    Some(match function {
        PhysicalFunction::Add => ArithmeticOperator::Add,
        PhysicalFunction::Subtract => ArithmeticOperator::Subtract,
        PhysicalFunction::Multiply => ArithmeticOperator::Multiply,
        PhysicalFunction::Divide => ArithmeticOperator::Divide,
        _ => return None,
    })
}

fn comparison_operator(function: PhysicalFunction) -> Option<ComparisonOperator> {
    Some(match function {
        PhysicalFunction::Less => ComparisonOperator::Less,
        PhysicalFunction::LessOrEqual => ComparisonOperator::LessOrEqual,
        PhysicalFunction::Greater => ComparisonOperator::Greater,
        PhysicalFunction::GreaterOrEqual => ComparisonOperator::GreaterOrEqual,
        PhysicalFunction::Equal => ComparisonOperator::Equal,
        PhysicalFunction::NotEqual => ComparisonOperator::NotEqual,
        _ => return None,
    })
}

fn unsupported(function: PhysicalFunction, inputs: &[PhysicalType]) -> anyhow::Error {
    anyhow::anyhow!(
        "unsupported physical signature for {}: {inputs:?}",
        entry(function).name
    )
}

/// Instantiate one concrete physical expression for an exact input signature.
pub fn build_physical_expression(
    function: PhysicalFunction,
    inputs: &[PhysicalType],
) -> anyhow::Result<Box<dyn Expression>> {
    let name = entry(function).name;

    if let Some(operator) = arithmetic_operator(function) {
        let [left, right] = inputs else {
            return Err(unsupported(function, inputs));
        };
        let output = numeric_output(left, right).ok_or_else(|| unsupported(function, inputs))?;
        return Ok(Box::new(build_numeric_binary_expression(
            name,
            operator,
            left.clone(),
            right.clone(),
            output,
        )));
    }

    if let Some(operator) = comparison_operator(function) {
        let [left, right] = inputs else {
            return Err(unsupported(function, inputs));
        };
        if let Some(common) = numeric_output(left, right) {
            return Ok(Box::new(build_numeric_comparison_expression(
                name,
                operator,
                left.clone(),
                right.clone(),
                common,
            )));
        }
        if *left == PhysicalType::String && *right == PhysicalType::String {
            return Ok(Box::new(build_string_comparison_expression(name, operator)));
        }
        if matches!(
            operator,
            ComparisonOperator::Equal | ComparisonOperator::NotEqual
        ) && *left == PhysicalType::Bool
            && *right == PhysicalType::Bool
        {
            return Ok(Box::new(build_bool_comparison_expression(name, operator)));
        }
        return Err(unsupported(function, inputs));
    }

    let expression: Box<dyn Expression> = match function {
        PhysicalFunction::Negate => {
            let [input] = inputs else {
                return Err(unsupported(function, inputs));
            };
            numeric_output(input, input).ok_or_else(|| unsupported(function, inputs))?;
            Box::new(build_numeric_neg_expression(name, input.clone()))
        }
        PhysicalFunction::Clamp => {
            let [value, lower, upper] = inputs else {
                return Err(unsupported(function, inputs));
            };
            let pair = numeric_output(value, lower).ok_or_else(|| unsupported(function, inputs))?;
            let output =
                numeric_output(&pair, upper).ok_or_else(|| unsupported(function, inputs))?;
            Box::new(build_numeric_clamp_expression(
                name,
                [value.clone(), lower.clone(), upper.clone()],
                output,
            ))
        }
        PhysicalFunction::BooleanAnd | PhysicalFunction::BooleanOr => {
            if inputs != [PhysicalType::Bool, PhysicalType::Bool] {
                return Err(unsupported(function, inputs));
            }
            let operator = if function == PhysicalFunction::BooleanAnd {
                BooleanOperator::And
            } else {
                BooleanOperator::Or
            };
            Box::new(build_boolean_binary_expression(name, operator))
        }
        PhysicalFunction::BooleanNot => {
            if inputs != [PhysicalType::Bool] {
                return Err(unsupported(function, inputs));
            }
            Box::new(build_boolean_not_expression(name))
        }
        PhysicalFunction::StringConcat | PhysicalFunction::StringContains => {
            if inputs != [PhysicalType::String, PhysicalType::String] {
                return Err(unsupported(function, inputs));
            }
            if function == PhysicalFunction::StringConcat {
                Box::new(build_string_concat_expression())
            } else {
                Box::new(build_string_contains_expression(name))
            }
        }
        _ => unreachable!("arithmetic and comparison functions returned above"),
    };
    Ok(expression)
}
