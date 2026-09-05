#![forbid(unsafe_code)]

mod array;
mod data_type;
mod decimal;
mod physical_type;
mod scalar;
mod variant_catalog;

// Enable later modules only when their checkpoints introduce them.
// Checkpoint 2:
// mod column;
// Checkpoint 3:
// mod expression;

pub use array::*;
pub use physical_type::*;
pub use scalar::*;
// Checkpoint 1: export the logical and Decimal definitions after implementing them.
// pub use data_type::*;
// pub use decimal::*;
// Checkpoint 2:
// pub use column::*;
// Checkpoint 3:
// pub use expression::*;
