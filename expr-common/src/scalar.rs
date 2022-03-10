// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Contains types for single values
//!
//! This crate contains two reciprocal traits -- Scalar and ScalarRef. As it is named, Scalar is an
//! owned value of ScalarRef, and ScalarRef is a reference to Scalar. We associate Scalar and
//! ScalarRef with Array types, and present examples on how to use these traits.

mod impls;
mod list;

pub use list::*;
use rust_decimal::Decimal;

use crate::array::Array;

/// An owned single value.
///
/// For example, `i32`, `String` both implements [`Scalar`].
pub trait Scalar:
    std::fmt::Debug + Clone + Send + Sync + 'static + TryFrom<ScalarImpl> + Into<ScalarImpl>
where
    for<'a> Self::ArrayType: Array<RefItem<'a> = Self::RefType<'a>>,
{
    /// The corresponding [`Array`] type.
    type ArrayType: Array<OwnedItem = Self>;

    /// The corresponding [`ScalarRef`] type.
    type RefType<'a>: ScalarRef<'a, ScalarType = Self, ArrayType = Self::ArrayType>;

    /// Get a reference of the current value.
    fn as_scalar_ref(&self) -> Self::RefType<'_>;

    /// Upcast GAT type's lifetime.
    fn upcast_gat<'short, 'long: 'short>(long: Self::RefType<'long>) -> Self::RefType<'short>;
}

/// An borrowed value.
///
/// For example, `i32`, `&str` both implements [`ScalarRef`].
pub trait ScalarRef<'a>:
    std::fmt::Debug + Clone + Copy + Send + 'a + TryFrom<ScalarRefImpl<'a>> + Into<ScalarRefImpl<'a>>
{
    /// The corresponding [`Array`] type.
    type ArrayType: Array<RefItem<'a> = Self>;

    /// The corresponding [`Scalar`] type.
    type ScalarType: Scalar<RefType<'a> = Self>;

    /// Convert the reference into an owned value.
    fn to_owned_scalar(&self) -> Self::ScalarType;
}

/// Encapsules all variants of [`Scalar`]
#[derive(Debug, Clone)]
pub enum ScalarImpl {
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Bool(bool),
    String(String),
    Decimal(Decimal),
    List(List),
}

impl PartialEq for ScalarImpl {
    fn eq(&self, other: &Self) -> bool {
        use ScalarImpl::*;
        match (self, other) {
            (Int16(a), Int16(b)) => a.eq(b),
            (Int32(a), Int32(b)) => a.eq(b),
            (Int64(a), Int64(b)) => a.eq(b),
            (Float32(a), Float32(b)) => a.eq(b),
            (Float64(a), Float64(b)) => a.eq(b),
            (Bool(a), Bool(b)) => a.eq(b),
            (String(a), String(b)) => a.eq(b),
            (Decimal(a), Decimal(b)) => a.eq(b),
            (List(_), List(_)) => unimplemented!("list eq is not implemented"),
            _ => false,
        }
    }
}

/// Encapsules all variants of [`ScalarRef`]
#[derive(Debug, Clone, Copy)]
pub enum ScalarRefImpl<'a> {
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Bool(bool),
    String(&'a str),
    Decimal(Decimal),
    List(ListRef<'a>),
}

impl<'a> PartialEq for ScalarRefImpl<'a> {
    fn eq(&self, other: &Self) -> bool {
        use ScalarRefImpl::*;
        match (self, other) {
            (Int16(a), Int16(b)) => a.eq(b),
            (Int32(a), Int32(b)) => a.eq(b),
            (Int64(a), Int64(b)) => a.eq(b),
            (Float32(a), Float32(b)) => a.eq(b),
            (Float64(a), Float64(b)) => a.eq(b),
            (Bool(a), Bool(b)) => a.eq(b),
            (String(a), String(b)) => a.eq(b),
            (Decimal(a), Decimal(b)) => a.eq(b),
            (List(_), List(_)) => unimplemented!("list eq is not implemented"),
            _ => false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::*;

    // These are two examples of using generics over array and scalar.
    //
    // These functions work for all kinds of scalars, no matter `String` or `i32`.

    /// Build an array from a vector of repeated data
    fn build_array_repeated<A: Array>(item: A::RefItem<'_>, len: usize) -> A {
        let mut builder = A::Builder::with_capacity(len);
        for _ in 0..len {
            builder.push(Some(item));
        }
        builder.finish()
    }

    /// Build an array from a vector of repeated owned data
    fn build_array_repeated_owned<A: Array>(item: A::OwnedItem, len: usize) -> A {
        let mut builder = A::Builder::with_capacity(len);
        for _ in 0..len {
            builder.push(Some(item.as_scalar_ref()));
        }
        builder.finish()
    }

    /// Test if an array has the same repeating content
    fn check_array_eq<'a, A: Array>(array: &'a A, item: A::RefItem<'a>)
    where
        A::RefItem<'a>: PartialEq,
    {
        for a in array.iter() {
            assert_eq!(a, Some(item));
        }
    }

    #[test]
    fn test_build_int32_repeat_array() {
        let array = build_array_repeated::<I32Array>(1, 233);
        check_array_eq(&array, 1);
        let array = build_array_repeated_owned::<I32Array>(1, 233);
        check_array_eq(&array, 1);
    }

    #[test]
    fn test_build_string_repeat_array() {
        let array = build_array_repeated::<StringArray>("233", 5);
        check_array_eq(&array, "233");
        let array = build_array_repeated_owned::<StringArray>("233".to_string(), 5);
        check_array_eq(&array, "233");
    }

    #[test]
    fn test_try_from_into() {
        let i: i32 = 2333;
        let j: ScalarImpl = i.into();
        let k: ScalarRefImpl = i.into();
        let i1: i32 = j.try_into().unwrap();
        let i2: i32 = k.try_into().unwrap();
        assert_eq!(i1, i);
        assert_eq!(i2, i);
    }
}
