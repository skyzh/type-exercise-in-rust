#![forbid(unsafe_code)]

mod array;
mod physical_type;
mod scalar;
mod variant_catalog;

// Enable these core modules as their chapters introduce them.
// Day 2:
// mod data_type;
// mod decimal;
// Day 3:
// mod column;
// Day 4:
// mod expression;
// Day 5:
// mod promotion;

pub use array::*;
pub use physical_type::*;
pub use scalar::*;
// Day 2:
// pub use data_type::*;
// pub use decimal::*;
// Day 3:
// pub use column::*;
// Day 4:
// pub use expression::*;
// Day 5:
// pub use promotion::*;
