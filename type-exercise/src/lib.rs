#![forbid(unsafe_code)]

mod array;
mod physical_type;
mod scalar;

#[cfg(test)]
mod tests;

pub use array::*;
pub use physical_type::*;
pub use scalar::*;
