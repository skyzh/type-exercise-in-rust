// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

use anyhow::Result;

use crate::array::ArrayImpl;

/// A trait over all expressions -- unary, binary, etc.
pub trait Expression {
    /// Evaluate an expression with run-time number of [`ArrayImpl`]s.
    fn eval_expr(&self, data: &[&ArrayImpl]) -> Result<ArrayImpl>;
}
