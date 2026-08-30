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
