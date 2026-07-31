// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

use anyhow::Result;

use crate::array::ArrayImpl;
use crate::column::ColumnViewImpl;

/// A trait over all expressions -- unary, binary, etc.
pub trait Expression {
    /// Evaluate type-erased input views.
    fn eval(&self, data: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl>;

    /// Compatibility adapter for callers that only have regular arrays.
    fn eval_expr(&self, data: &[&ArrayImpl]) -> Result<ArrayImpl> {
        let views = data
            .iter()
            .map(|array| ColumnViewImpl::array(array))
            .collect::<Vec<_>>();
        self.eval(&views)
    }
}
