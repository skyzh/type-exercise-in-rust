// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Implements compare functions for [`Array`] types

#![allow(dead_code)]

use std::cmp::Ordering;

use crate::scalar::Scalar;

/// Return if `i1 < i2`. Note that `i1` and `i2` could be different types. This
/// function will automatically cast them into `C` type.
///
/// * `I1`: left input type.
/// * `I2`: right input type.
/// * `C`: cast type.
pub fn cmp_le<I1: Scalar, I2: Scalar, C: Scalar>(i1: I1::RefType<'_>, i2: I2::RefType<'_>) -> bool
where
    for<'a> I1::RefType<'a>: Into<C::RefType<'a>>,
    for<'a> I2::RefType<'a>: Into<C::RefType<'a>>,
    for<'a> C::RefType<'a>: PartialOrd,
{
    let i1 = I1::upcast_gat(i1);
    let i2 = I2::upcast_gat(i2);
    i1.into().partial_cmp(&i2.into()).unwrap() == Ordering::Less
}

/// Return if `i1 > i2`. Note that `i1` and `i2` could be different types. This
/// function will automatically cast them into `C` type.
///
/// * `I1`: left input type.
/// * `I2`: right input type.
/// * `C`: cast type.
pub fn cmp_ge<I1: Scalar, I2: Scalar, C: Scalar>(i1: I1::RefType<'_>, i2: I2::RefType<'_>) -> bool
where
    for<'a> I1::RefType<'a>: Into<C::RefType<'a>>,
    for<'a> I2::RefType<'a>: Into<C::RefType<'a>>,
    for<'a> C::RefType<'a>: PartialOrd,
{
    let i1 = I1::upcast_gat(i1);
    let i2 = I2::upcast_gat(i2);
    i1.into().partial_cmp(&i2.into()).unwrap() == Ordering::Greater
}

/// Return if `i1 == i2`. Note that `i1` and `i2` could be different types. This
/// function will automatically cast them into `C` type.
///
/// * `I1`: left input type.
/// * `I2`: right input type.
/// * `C`: cast type.
pub fn cmp_eq<I1: Scalar, I2: Scalar, C: Scalar>(i1: I1::RefType<'_>, i2: I2::RefType<'_>) -> bool
where
    for<'a> I1::RefType<'a>: Into<C::RefType<'a>>,
    for<'a> I2::RefType<'a>: Into<C::RefType<'a>>,
    for<'a> C::RefType<'a>: PartialEq,
{
    let i1 = I1::upcast_gat(i1);
    let i2 = I2::upcast_gat(i2);
    i1.into().eq(&i2.into())
}

/// Return if `i1 != i2`. Note that `i1` and `i2` could be different types. This
/// function will automatically cast them into `C` type.
///
/// * `I1`: left input type.
/// * `I2`: right input type.
/// * `C`: cast type.
pub fn cmp_ne<I1: Scalar, I2: Scalar, C: Scalar>(i1: I1::RefType<'_>, i2: I2::RefType<'_>) -> bool
where
    for<'a> I1::RefType<'a>: Into<C::RefType<'a>>,
    for<'a> I2::RefType<'a>: Into<C::RefType<'a>>,
    for<'a> C::RefType<'a>: PartialEq,
{
    let i1 = I1::upcast_gat(i1);
    let i2 = I2::upcast_gat(i2);
    !i1.into().eq(&i2.into())
}
