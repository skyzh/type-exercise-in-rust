//! Implements string functions for [`Array`] types

use super::vectorize::BinaryExprFunc;
use crate::array::{BoolArray, StringArray};

/// Checks if `i1.contains(i2)` for two string inputs.
pub struct ExprStrContains;

impl BinaryExprFunc<StringArray, StringArray, BoolArray> for ExprStrContains {
    fn eval(&self, i1: &str, i2: &str) -> bool {
        i1.contains(i2)
    }
}
