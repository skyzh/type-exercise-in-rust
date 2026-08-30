#![forbid(unsafe_code)]

mod array;
mod column;
mod data_type;
mod decimal;
mod physical_type;
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
pub use column::{ColumnView, ColumnViewImpl};
pub use data_type::*;
pub use decimal::*;
pub use physical_type::*;
pub use scalar::*;
