use type_exercise_checkpoint_04_core::{
    ArrayImpl, ColumnViewImpl, evaluate_binary, evaluate_ternary, evaluate_unary,
};

/// Instantiate the shared binary fallback for one lossless mixed-width addition.
pub fn add_i16_i32(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
) -> anyhow::Result<ArrayImpl> {
    evaluate_binary::<i16, i32, i32, _>(left, right, |left, right| i32::from(left) + right)
}

/// Instantiate the shared unary fallback for signed Int32 negation.
pub fn negate_i32(input: ColumnViewImpl<'_>) -> anyhow::Result<ArrayImpl> {
    evaluate_unary::<i32, i32, _>(input, i32::wrapping_neg)
}

/// Instantiate the shared ternary fallback for Int32 clamp.
pub fn clamp_i32(
    value: ColumnViewImpl<'_>,
    lower: ColumnViewImpl<'_>,
    upper: ColumnViewImpl<'_>,
) -> anyhow::Result<ArrayImpl> {
    evaluate_ternary::<i32, i32, i32, i32, _>(value, lower, upper, i32::clamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use type_exercise_checkpoint_04_core::{Array, I16Array, I32Array, ScalarRefImpl};

    #[test]
    fn concrete_instantiations_delegate_complete_batches_to_core() {
        let left: ArrayImpl = I16Array::from_slice(&[Some(2), None]).into();
        let output = add_i16_i32(
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
        )
        .unwrap();
        assert_eq!(
            I32Array::try_from(output)
                .unwrap()
                .iter()
                .collect::<Vec<_>>(),
            vec![Some(6), None]
        );
    }
}
