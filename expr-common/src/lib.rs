// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! A typed database expression framework in Rust.
//!
//! The framework keeps physical array layouts, borrowed scalar values, column views, and runtime
//! type erasure separate. Generic machinery is used where it pays off—especially numeric and
//! comparison kernels—while data-type-specific expressions can use the same execution interface.

pub mod array;
pub mod column;
pub mod datatype;
pub mod expr;
mod macros;
pub mod scalar;

use array::PhysicalType;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("type mismatch: expected {0:?}, got {1:?}")]
pub struct TypeMismatch(pub PhysicalType, pub PhysicalType);

pub use rust_decimal::Decimal;
