//! Transactional writer-backed variable-width evaluation.

use crate::{Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, PhysicalType};

use super::validate_expression_inputs;

/// Lift one strict string scalar function into the transactional writer loop.
pub fn evaluate_writer_binary<F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: for<'a> Fn(&str, &str, crate::Writer<'a>) -> crate::WriterUsed<'a>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[PhysicalType::String, PhysicalType::String],
    )?;
    let left = ColumnView::<String>::try_from(left)?;
    let right = ColumnView::<String>::try_from(right)?;
    let mut output = <crate::StringArray as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => {
                function(left, right, output.writer()).into_builder();
            }
            _ => output.push_null(),
        }
    }
    Ok(output.finish().into())
}
