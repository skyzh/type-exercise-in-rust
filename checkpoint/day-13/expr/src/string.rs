//! Variable-width scalar operations and their writer-backed adapters.

use crate::{
    ArrayImpl, BinaryExpression, ColumnViewImpl, PhysicalType, Writer, WriterUsed,
    evaluate_writer_binary,
};

fn concat_strings<'a>(left: &str, right: &str, writer: Writer<'a>) -> WriterUsed<'a> {
    writer.write(|value| {
        value.push_str(left);
        value.push_str(right);
    })
}

fn evaluate_string_concat(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    evaluate_writer_binary(inputs[0].clone(), inputs[1].clone(), concat_strings)
}

pub(crate) fn build_string_concat_expression() -> BinaryExpression {
    BinaryExpression::new(
        "string_concat",
        [PhysicalType::String, PhysicalType::String],
        PhysicalType::String,
        evaluate_string_concat,
    )
}

use crate::{ComparisonOperator, Expression, evaluate_borrowed_binary};

pub(crate) enum StringOperator {
    Compare(ComparisonOperator),
    Contains,
}

pub(crate) struct StringBinaryExpression {
    name: &'static str,
    input_types: [PhysicalType; 2],
    operator: StringOperator,
}

impl StringBinaryExpression {
    pub(crate) fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        crate::validate_expression_inputs(inputs, &self.input_types)?;
        let left = inputs[0].clone();
        let right = inputs[1].clone();
        match self.operator {
            StringOperator::Contains => {
                evaluate_borrowed_binary::<String, String, bool, _>(left, right, str::contains)
            }
            StringOperator::Compare(ComparisonOperator::Less) => {
                evaluate_borrowed_binary::<String, String, bool, _>(
                    left,
                    right,
                    crate::numeric::less,
                )
            }
            StringOperator::Compare(ComparisonOperator::LessOrEqual) => {
                evaluate_borrowed_binary::<String, String, bool, _>(
                    left,
                    right,
                    crate::numeric::less_or_equal,
                )
            }
            StringOperator::Compare(ComparisonOperator::Greater) => {
                evaluate_borrowed_binary::<String, String, bool, _>(
                    left,
                    right,
                    crate::numeric::greater,
                )
            }
            StringOperator::Compare(ComparisonOperator::GreaterOrEqual) => {
                evaluate_borrowed_binary::<String, String, bool, _>(
                    left,
                    right,
                    crate::numeric::greater_or_equal,
                )
            }
            StringOperator::Compare(ComparisonOperator::Equal) => {
                evaluate_borrowed_binary::<String, String, bool, _>(
                    left,
                    right,
                    crate::numeric::equal,
                )
            }
            StringOperator::Compare(ComparisonOperator::NotEqual) => {
                evaluate_borrowed_binary::<String, String, bool, _>(
                    left,
                    right,
                    crate::numeric::not_equal,
                )
            }
        }
    }
}

impl Expression for StringBinaryExpression {
    fn name(&self) -> &'static str {
        self.name
    }

    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> PhysicalType {
        PhysicalType::Bool
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate(inputs)
    }
}

pub(crate) fn build_string_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
) -> StringBinaryExpression {
    StringBinaryExpression {
        name,
        input_types: [PhysicalType::String, PhysicalType::String],
        operator: StringOperator::Compare(operator),
    }
}

pub(crate) fn build_string_contains_expression(name: &'static str) -> StringBinaryExpression {
    StringBinaryExpression {
        name,
        input_types: [PhysicalType::String, PhysicalType::String],
        operator: StringOperator::Contains,
    }
}
