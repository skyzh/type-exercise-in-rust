// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Implements compare functions for [`Array`] types

#![allow(dead_code)]

use std::cmp::Ordering;

use crate::array::*;

/// Return if `i1 < i2`. Note that `i1` and `i2` could be different types. This
/// function will automatically cast them into `C` type.
///
/// * `I1`: left input type.
/// * `I2`: right input type.
/// * `C`: cast type.
pub fn cmp_le<'a, I1: Array, I2: Array, C: Array + 'static>(
    i1: I1::RefItem<'a>,
    i2: I2::RefItem<'a>,
) -> bool
where
    I1::RefItem<'a>: Into<C::RefItem<'a>>,
    I2::RefItem<'a>: Into<C::RefItem<'a>>,
    C::RefItem<'a>: PartialOrd,
{
    i1.into().partial_cmp(&i2.into()).unwrap() == Ordering::Less
}

/// Return if `i1 > i2`. Note that `i1` and `i2` could be different types. This
/// function will automatically cast them into `C` type.
///
/// * `I1`: left input type.
/// * `I2`: right input type.
/// * `C`: cast type.
pub fn cmp_ge<'a, I1: Array, I2: Array, C: Array + 'static>(
    i1: I1::RefItem<'a>,
    i2: I2::RefItem<'a>,
) -> bool
where
    I1::RefItem<'a>: Into<C::RefItem<'a>>,
    I2::RefItem<'a>: Into<C::RefItem<'a>>,
    C::RefItem<'a>: PartialOrd,
{
    i1.into().partial_cmp(&i2.into()).unwrap() == Ordering::Greater
}
