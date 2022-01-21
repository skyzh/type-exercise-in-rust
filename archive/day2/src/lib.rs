//! Type exercise in Rust
//!
//! This is a short lecture on how to use the Rust type system to build necessary components in a
//! database system. The lecture evolves around how Rust programmers (like me) build database
//! systems in the Rust programming language. We leverage the Rust type system to **minimize**
//! runtime cost and make our development process easier with **safe**, **nightly** Rust.

#![feature(generic_associated_types)]

pub mod array;
pub mod scalar;
