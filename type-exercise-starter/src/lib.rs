#![forbid(unsafe_code)]

mod array;
mod data_type;
mod decimal;
mod physical_type;
mod scalar;
mod variant_catalog;

// Day 3, checkpoint 1: uncomment after implementing `src/column.rs`.
// mod column;
// pub use column::{ColumnView, ColumnViewImpl};
// Day 4, checkpoints 1–2: uncomment after implementing the expression/operator skeletons.
// mod expression;
// mod operators;
// Day 5, checkpoint 1: uncomment after implementing numeric promotion.
// mod promotion;
// Day 7, checkpoint 1: uncomment after implementing three-valued Boolean logic.
// mod boolean_logic;
// Day 9, checkpoint 1: uncomment after implementing the binder and registry.
// mod binder;

#[cfg(test)]
mod tests;

pub use array::*;
pub use data_type::*;
pub use decimal::*;
pub use physical_type::*;
pub use scalar::*;
