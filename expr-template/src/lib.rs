// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

mod common;

#[rustfmt::skip]
mod r#gen;

pub use r#gen::{
    FnArgs1Expression as UnaryExpression, FnArgs2Expression as BinaryExpression, FnArgs3Expression,
    FnArgs4Expression, FnArgs5Expression,
};
