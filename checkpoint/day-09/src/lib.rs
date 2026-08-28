#![forbid(unsafe_code)]

mod array;
mod binder;
mod boolean_logic;
mod column;
mod data_type;
mod decimal;
mod expression;
mod operators;
mod physical_type;
mod promotion;
mod scalar;
mod variant_catalog;

// Day 3 adds and exports `column`.
// Day 4 adds and exports `expression` and `operators`.
// Day 5 adds and exports `promotion`.
// Day 7 adds and exports `boolean_logic`.
// Day 9 adds and exports `binder`.

#[cfg(test)]
mod tests;

pub use array::*;
pub use binder::*;
pub use boolean_logic::*;
pub use column::{ColumnView, ColumnViewImpl};
pub use data_type::*;
pub use decimal::*;
pub use expression::*;
pub use operators::*;
pub use physical_type::*;
pub use promotion::*;
pub use scalar::*;
