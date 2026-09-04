//! Int32 values/validity specialization, isolated from scalar semantics.

use bitvec::vec::BitVec;

use crate::column::RawI32Column;
use crate::{ArrayImpl, ColumnViewImpl, I32Array, PhysicalType};

use super::{auto_vectorize_binary, validate_expression_inputs};

fn raw_array_array<F>(function: &F, left: &[i32], right: &[i32]) -> Vec<i32>
where
    F: Fn(i32, i32) -> i32,
{
    let mut output = Vec::with_capacity(left.len());
    for row in 0..left.len() {
        output.push(function(left[row], right[row]));
    }
    output
}

fn raw_array_constant<F>(function: &F, left: &[i32], right: i32) -> Vec<i32>
where
    F: Fn(i32, i32) -> i32,
{
    left.iter().map(|left| function(*left, right)).collect()
}

fn raw_constant_array<F>(function: &F, left: i32, right: &[i32]) -> Vec<i32>
where
    F: Fn(i32, i32) -> i32,
{
    right.iter().map(|right| function(left, *right)).collect()
}

fn raw_constant_constant<F>(function: &F, left: i32, right: i32, len: usize) -> Vec<i32>
where
    F: Fn(i32, i32) -> i32,
{
    if len == 0 {
        Vec::new()
    } else {
        vec![function(left, right); len]
    }
}

fn and_validity(left: RawI32Column<'_>, right: RawI32Column<'_>, len: usize) -> BitVec {
    match (left, right) {
        (
            RawI32Column::Array { validity: left, .. },
            RawI32Column::Array {
                validity: right, ..
            },
        ) => {
            let words = left
                .as_raw_slice()
                .iter()
                .zip(right.as_raw_slice())
                .map(|(left, right)| *left & *right)
                .collect();
            let mut validity = BitVec::from_vec(words);
            validity.truncate(len);
            validity
        }
        (RawI32Column::Array { validity, .. }, RawI32Column::Constant { valid: true, .. })
        | (RawI32Column::Constant { valid: true, .. }, RawI32Column::Array { validity, .. }) => {
            validity.clone()
        }
        (
            RawI32Column::Constant {
                valid: left_valid, ..
            },
            RawI32Column::Constant {
                valid: right_valid, ..
            },
        ) => BitVec::repeat(left_valid & right_valid, len),
        _ => BitVec::repeat(false, len),
    }
}

/// Evaluate one strict, total and infallible Int32 scalar operation through
/// raw values and validity. Indexed input deliberately uses the typed fallback.
pub fn auto_vectorize_primitive_i32<F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: Fn(i32, i32) -> i32,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[PhysicalType::Int32, PhysicalType::Int32],
    )?;
    if left.is_indexed() || right.is_indexed() {
        return auto_vectorize_binary::<i32, i32, i32, _>(left, right, function);
    }

    let left = left
        .as_raw_i32()
        .expect("validated non-indexed Int32 input");
    let right = right
        .as_raw_i32()
        .expect("validated non-indexed Int32 input");
    debug_assert_eq!(left.len(), right.len());
    let len = left.len();
    let values = match (left, right) {
        (RawI32Column::Array { values: left, .. }, RawI32Column::Array { values: right, .. }) => {
            raw_array_array(&function, left, right)
        }
        (RawI32Column::Array { values: left, .. }, RawI32Column::Constant { value: right, .. }) => {
            raw_array_constant(&function, left, right)
        }
        (RawI32Column::Constant { value: left, .. }, RawI32Column::Array { values: right, .. }) => {
            raw_constant_array(&function, left, right)
        }
        (
            RawI32Column::Constant { value: left, .. },
            RawI32Column::Constant { value: right, .. },
        ) => raw_constant_constant(&function, left, right, len),
    };
    let validity = and_validity(left, right, len);
    Ok(I32Array::from_raw_parts(values, validity).into())
}
