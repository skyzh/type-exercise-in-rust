// Copyright 2022-2026 Alex Chi. Licensed under Apache-2.0.

//! Generic numeric kernels.

use std::ops::Add;

use expr_common::scalar::Scalar;

/// Cast two supported numeric inputs to the planner-selected output type and add them.
pub fn add<I1, I2, O>(i1: I1::RefType<'_>, i2: I2::RefType<'_>) -> O
where
    I1: Scalar,
    I2: Scalar,
    O: Scalar + Add<Output = O>,
    for<'a> I1::RefType<'a>: Into<O>,
    for<'a> I2::RefType<'a>: Into<O>,
{
    i1.into() + i2.into()
}
