//! Variable-width scalar operations and their writer-backed adapters.

use crate::{
    ArrayImpl, BatchExpression, BatchKernel, ColumnViewImpl, PhysicalType, Writer, WriterUsed,
    evaluate_binary, evaluate_writer_binary,
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

pub(crate) fn build_string_concat_expression() -> BatchExpression<2> {
    BatchExpression::new(
        "string_concat",
        [PhysicalType::String, PhysicalType::String],
        PhysicalType::String,
        evaluate_string_concat,
    )
}

use crate::ComparisonOperator;

macro_rules! define_string_kernel {
    ($name:ident, $operation:path) => {
        fn $name(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
            evaluate_binary::<String, String, bool, _>(
                inputs[0].clone(),
                inputs[1].clone(),
                $operation,
            )
        }
    };
}

define_string_kernel!(evaluate_string_contains, str::contains);
define_string_kernel!(evaluate_string_less, crate::numeric::less);
define_string_kernel!(evaluate_string_less_or_equal, crate::numeric::less_or_equal);
define_string_kernel!(evaluate_string_greater, crate::numeric::greater);
define_string_kernel!(
    evaluate_string_greater_or_equal,
    crate::numeric::greater_or_equal
);
define_string_kernel!(evaluate_string_equal, crate::numeric::equal);
define_string_kernel!(evaluate_string_not_equal, crate::numeric::not_equal);

pub(crate) fn build_string_comparison_expression(
    name: &'static str,
    operator: ComparisonOperator,
) -> BatchExpression<2> {
    let kernel: BatchKernel = match operator {
        ComparisonOperator::Less => evaluate_string_less,
        ComparisonOperator::LessOrEqual => evaluate_string_less_or_equal,
        ComparisonOperator::Greater => evaluate_string_greater,
        ComparisonOperator::GreaterOrEqual => evaluate_string_greater_or_equal,
        ComparisonOperator::Equal => evaluate_string_equal,
        ComparisonOperator::NotEqual => evaluate_string_not_equal,
    };
    BatchExpression::new(
        name,
        [PhysicalType::String, PhysicalType::String],
        PhysicalType::Bool,
        kernel,
    )
}

pub(crate) fn build_string_contains_expression(name: &'static str) -> BatchExpression<2> {
    BatchExpression::new(
        name,
        [PhysicalType::String, PhysicalType::String],
        PhysicalType::Bool,
        evaluate_string_contains,
    )
}
