#![forbid(unsafe_code)]

mod binder;
mod boolean;
mod catalog;
mod numeric;
mod string;

pub use binder::*;
pub use boolean::*;
pub use catalog::*;
pub use numeric::*;
pub use type_exercise_checkpoint_10_core::*;

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
