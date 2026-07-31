// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Implements compare functions for [`Array`] types

use std::cmp::Ordering;

use expr_common::scalar::Scalar;

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
    for<'a, 'b> C::RefType<'a>: PartialOrd<C::RefType<'b>>,
{
    matches!(
        i1.into().partial_cmp(&i2.into()),
        Some(Ordering::Less | Ordering::Equal)
    )
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
    for<'a, 'b> C::RefType<'a>: PartialOrd<C::RefType<'b>>,
{
    matches!(
        i1.into().partial_cmp(&i2.into()),
        Some(Ordering::Greater | Ordering::Equal)
    )
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
    for<'a, 'b> C::RefType<'a>: PartialEq<C::RefType<'b>>,
{
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
    for<'a, 'b> C::RefType<'a>: PartialEq<C::RefType<'b>>,
{
    !i1.into().eq(&i2.into())
}
