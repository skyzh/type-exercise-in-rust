// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Type exercise in Rust
//!
//! This is a short lecture on how to use the Rust type system to build necessary components in a
//! database system. The lecture evolves around how Rust programmers (like me) build database
//! systems in the Rust programming language. We leverage the Rust type system to **minimize**
//! runtime cost and make our development process easier with **safe**, **nightly** Rust.

#![feature(generic_associated_types)]
#![feature(trace_macros)]
#![feature(trusted_len)]

pub mod array;
pub mod column;
pub mod datatype;
pub mod expr;
mod macros;
pub mod scalar;

use array::PhysicalType;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Type mismatch on conversion: expected {0:?}, get {1:?}")]
pub struct TypeMismatch(PhysicalType, PhysicalType);
