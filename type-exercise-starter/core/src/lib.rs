#![forbid(unsafe_code)]

mod array;
mod physical_type;
mod scalar;
mod variant_catalog;

// Enable these core modules as their chapters introduce them.
// Chapter 1:
// mod data_type;
// mod decimal;
// Chapter 2:
// mod column;
// Chapter 3:
// mod expression;
// Chapter 8:
// mod promotion;

pub use array::*;
pub use physical_type::*;
pub use scalar::*;
// Chapter 1:
// pub use data_type::*;
// pub use decimal::*;
// Chapter 2:
// pub use column::*;
// Chapter 3:
// pub use expression::*;
// Chapter 8:
// pub use promotion::*;
