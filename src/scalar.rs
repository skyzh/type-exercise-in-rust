//! Contains types for single values
//!
//! This crate contains two reciprocal traits -- Scalar and ScalarRef. As it is named, Scalar is an
//! owned value of ScalarRef, and ScalarRef is a reference to Scalar. We associate Scalar and
//! ScalarRef with Array types, and present examples on how to use these traits.

use crate::array::{Array, F32Array, I32Array, StringArray};

/// An owned single value.
///  
/// For example, `i32`, `String` both implements [`Scalar`].
pub trait Scalar: std::fmt::Debug + Clone + Send + Sync + 'static {
    /// The corresponding [`Array`] type.
    type ArrayType: Array<OwnedItem = Self>;

    /// The corresponding [`ScalarRef`] type.
    type RefType<'a>: ScalarRef<'a, ScalarType = Self, ArrayType = Self::ArrayType>
    where
        Self: 'a;

    /// Get a reference of the current value.
    fn as_scalar_ref(&self) -> Self::RefType<'_>;
}

/// An borrowed value.
///
/// For example, `i32`, `&str` both implements [`ScalarRef`].
pub trait ScalarRef<'a>: std::fmt::Debug + Clone + Copy + Send + 'a {
    /// The corresponding [`Array`] type.
    type ArrayType: Array<RefItem<'a> = Self>;

    /// The corresponding [`Scalar`] type.
    type ScalarType: Scalar<RefType<'a> = Self>;

    /// Convert the reference into an owned value.
    fn to_owned_scalar(&self) -> Self::ScalarType;
}

/// Implement [`Scalar`] for `i32`. Note that `i32` is both [`Scalar`] and [`ScalarRef`].
impl Scalar for i32 {
    type ArrayType = I32Array;
    type RefType<'a> = i32;

    fn as_scalar_ref(&self) -> i32 {
        *self
    }
}

/// Implement [`ScalarRef`] for `i32`. Note that `i32` is both [`Scalar`] and [`ScalarRef`].
impl<'a> ScalarRef<'a> for i32 {
    type ArrayType = I32Array;
    type ScalarType = i32;

    fn to_owned_scalar(&self) -> i32 {
        *self
    }
}

/// Implement [`Scalar`] for `f32`. Note that `f32` is both [`Scalar`] and [`ScalarRef`].
impl Scalar for f32 {
    type ArrayType = F32Array;
    type RefType<'a> = f32;

    fn as_scalar_ref(&self) -> f32 {
        *self
    }
}

/// Implement [`ScalarRef`] for `f32`. Note that `f32` is both [`Scalar`] and [`ScalarRef`].
impl<'a> ScalarRef<'a> for f32 {
    type ArrayType = F32Array;
    type ScalarType = f32;

    fn to_owned_scalar(&self) -> f32 {
        *self
    }
}

/// Implement [`Scalar`] for `String`.
impl Scalar for String {
    type ArrayType = StringArray;
    type RefType<'a> = &'a str;

    fn as_scalar_ref(&self) -> &str {
        self.as_str()
    }
}

/// Implement [`ScalarRef`] for `&str`.
impl<'a> ScalarRef<'a> for &'a str {
    type ArrayType = StringArray;
    type ScalarType = String;

    fn to_owned_scalar(&self) -> String {
        self.to_string()
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
}
