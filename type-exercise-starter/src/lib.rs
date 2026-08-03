#![forbid(unsafe_code)]

mod scalar;

#[cfg(test)]
mod tests;

pub use scalar::ScalarImpl;
