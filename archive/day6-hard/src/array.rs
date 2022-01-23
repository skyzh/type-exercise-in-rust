// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Contains array types for the system
//!
//! This crate contains two category of structs -- ArrayBuilder and Array. Developers may use
//! ArrayBuilder to create an Array. ArrayBuilder and Array are reciprocal traits. We can associate
//! an Array with an ArrayBuilder at compile time. This module also contains examples on how to use
//! generics around the Array and ArrayBuilder.

mod impls;
mod iterator;
mod primitive_array;
mod string_array;

pub use iterator::*;
pub use primitive_array::*;
pub use string_array::*;

use crate::scalar::{Scalar, ScalarRef};
use crate::TypeMismatch;

/// [`Array`] is a collection of data of the same type.
pub trait Array:
    Send + Sync + Sized + 'static + TryFrom<ArrayImpl, Error = TypeMismatch> + Into<ArrayImpl>
where
    for<'a> Self::OwnedItem: Scalar<RefType<'a> = Self::RefItem<'a>>,
{
    /// The corresponding [`ArrayBuilder`] of this [`Array`].
    ///
    /// We constriant the associated type so that `Self::Builder::Array = Self`.
    type Builder: ArrayBuilder<Array = Self>;

    /// The owned item of this array.
    type OwnedItem: Scalar<ArrayType = Self>;

    /// Type of the item that can be retrieved from the [`Array`]. For example, we can get a `i32`
    /// from [`I32Array`], while [`StringArray`] produces a `&str`. As we need a lifetime that is
    /// the same as `self` for `&str`, we use GAT here.
    type RefItem<'a>: ScalarRef<'a, ScalarType = Self::OwnedItem, ArrayType = Self>
    where
        Self: 'a;

    /// Retrieve a reference to value.
    fn get(&self, idx: usize) -> Option<Self::RefItem<'_>>;

    /// Number of items of array.
    fn len(&self) -> usize;

    /// Indicates whether this array is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get iterator of this array.
    fn iter(&self) -> ArrayIterator<Self>;

    /// Bulid array from slice
    fn from_slice(data: &[Option<Self::RefItem<'_>>]) -> Self {
        let mut builder = Self::Builder::with_capacity(data.len());
        for item in data {
            builder.push(*item);
        }
        builder.finish()
    }
}

/// [`ArrayBuilder`] builds an [`Array`].
pub trait ArrayBuilder {
    /// The corresponding [`Array`] of this [`ArrayBuilder`].
    ///
    /// Here we use associated type to constraint the [`Array`] type of this builder, so that
    /// `Self::Array::Builder == Self`. This property is very useful when constructing generic
    /// functions, and may help a lot when implementing expressions.
    type Array: Array<Builder = Self>;

    /// Create a new builder with `capacity`.
    fn with_capacity(capacity: usize) -> Self;

    /// Append a value to builder.
    fn push(&mut self, value: Option<<Self::Array as Array>::RefItem<'_>>);

    /// Finish build and return a new array.
    fn finish(self) -> Self::Array;
}

/// Encapsules all variants of array in this library.
pub enum ArrayImpl {
    Int16(I16Array),
    Int32(I32Array),
    Int64(I64Array),
    Float32(F32Array),
    Float64(F64Array),
    Bool(BoolArray),
    String(StringArray),
}

/// Encapsules all variants of array builders in this library.
pub enum ArrayBuilderImpl {
    Int16(I16ArrayBuilder),
    Int32(I32ArrayBuilder),
    Int64(I64ArrayBuilder),
    Float32(F32ArrayBuilder),
    Float64(F64ArrayBuilder),
    Bool(BoolArrayBuilder),
    String(StringArrayBuilder),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TypeMismatch;

    // These are two examples of using generics over array.
    //
    // These functions work for all kinds of array, no matter fixed-length arrays like `I32Array`,
    // or variable-length ones like `StringArray`.

    /// Build an array from a vector of data
    fn build_array_from_vec<A: Array>(items: &[Option<A::RefItem<'_>>]) -> A {
        let mut builder = A::Builder::with_capacity(items.len());
        for item in items {
            builder.push(*item);
        }
        builder.finish()
    }

    /// Test if an array has the same content as a vector
    fn check_array_eq<'a, A: Array>(array: &'a A, vec: &[Option<A::RefItem<'a>>])
    where
        A::RefItem<'a>: PartialEq,
    {
        for (a, b) in array.iter().zip(vec.iter()) {
            assert_eq!(&a, b);
        }
    }

    #[test]
    fn test_build_int32_array() {
        let data = vec![Some(1), Some(2), Some(3), None, Some(5)];
        let array = build_array_from_vec::<I32Array>(&data[..]);
        check_array_eq(&array, &data[..]);
    }

    #[test]
    fn test_build_string_array() {
        let data = vec![Some("1"), Some("2"), Some("3"), None, Some("5"), Some("")];
        let array = build_array_from_vec::<StringArray>(&data[..]);
        check_array_eq(&array, &data[..]);
    }

    fn add_i32(i1: i32, i2: i32) -> i32 {
        i1 + i2
    }

    fn add_i32_vec(i1: I32Array, i2: I32Array) -> I32Array {
        let mut builder = I32ArrayBuilder::with_capacity(i1.len());
        for (a, b) in i1.iter().zip(i2.iter()) {
            builder.push(a.and_then(|a| b.map(|b| add_i32(a, b))));
        }
        builder.finish()
    }

    fn add_i32_wrapper(i1: ArrayImpl, i2: ArrayImpl) -> Result<ArrayImpl, TypeMismatch> {
        Ok(add_i32_vec(i1.try_into()?, i2.try_into()?).into())
    }

    #[test]
    fn test_add_array() {
        check_array_eq::<I32Array>(
            &add_i32_wrapper(
                I32Array::from_slice(&[Some(1), Some(2), Some(3), None]).into(),
                I32Array::from_slice(&[Some(1), Some(2), None, Some(4)]).into(),
            )
            .unwrap()
            .try_into()
            .unwrap(),
            &[Some(2), Some(4), None, None],
        );

        let result = add_i32_wrapper(
            StringArray::from_slice(&[Some("1"), Some("2"), Some("3"), None]).into(),
            I32Array::from_slice(&[Some(1), Some(2), None, Some(4)]).into(),
        );
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err.0, "Int32");
            assert_eq!(err.1, "String");
        }
    }
}
