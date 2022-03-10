// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

use std::any::Any;

use super::all_arrays::*;
use super::{
    Array, ArrayBuilder, ArrayBuilderImpl, ArrayImpl, ArrayImplRef, BoxedArray, PhysicalType,
};
use crate::macros::for_all_variants;
use crate::scalar::ScalarRefImpl;

pub trait PhysicalTypeOf {
    fn physical_type(&self) -> PhysicalType;
}

/// The object-safe array trait.
pub trait DynArray: Any + PhysicalTypeOf + 'static + Send + Sync + std::fmt::Debug {
    /// Get the array builder of the current array.
    fn new_builder(&self, capacity: usize) -> ArrayBuilderImpl;

    /// Get the value at the given index.
    fn get(&self, idx: usize) -> Option<ScalarRefImpl<'_>>;

    /// Number of items of array.
    fn len(&self) -> usize;

    /// Number of items of array.
    fn is_empty(&self) -> bool;

    /// Convert self into [`Any`].
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Convert self as [`Any`].
    fn as_any(&self) -> &dyn Any;

    /// Clone the current array
    fn boxed_clone(&self) -> Box<dyn DynArray>;
}

impl<A: Array + PhysicalTypeOf> DynArray for A
where
    A::Builder: Into<ArrayBuilderImpl>,
{
    fn new_builder(&self, capacity: usize) -> ArrayBuilderImpl {
        A::Builder::with_capacity(capacity).into()
    }

    fn get(&self, idx: usize) -> Option<ScalarRefImpl<'_>> {
        Array::get(self, idx).map(|x| x.into())
    }

    fn len(&self) -> usize {
        Array::len(self)
    }

    fn is_empty(&self) -> bool {
        Array::is_empty(self)
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn boxed_clone(&self) -> Box<dyn DynArray> {
        Box::new(self.clone())
    }
}

impl BoxedArray {
    /// Get the array builder of the current array.
    pub fn new_builder(&self, capacity: usize) -> ArrayBuilderImpl {
        self.0.new_builder(capacity)
    }

    /// Get the value at the given index.
    pub fn get(&self, idx: usize) -> Option<ScalarRefImpl<'_>> {
        self.0.get(idx)
    }

    /// Number of items of array.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Number of items of array.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Clone for BoxedArray {
    fn clone(&self) -> Self {
        Self(self.0.boxed_clone())
    }
}

/// Implements dispatch functions for [`BoxedArray`]
macro_rules! impl_boxed_array_dispatch {
    (
        [], $({ $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty }),*
    ) => {
        $(
            impl PhysicalTypeOf for $AbcArray {
                fn physical_type(&self) -> PhysicalType {
                    PhysicalType::$Abc
                }
            }
        )*

        impl ArrayImpl {
            /// Convert an [`ArrayImpl`] into [`BoxedArray`].
            pub fn into_boxed_array(self) -> BoxedArray {
                match self {
                    $(
                        Self::$Abc(a) => BoxedArray(Box::new(a)),
                    )*
                }
            }
        }

        impl BoxedArray {
            /// Convert an [`BoxedArray`] into [`ArrayImpl`]
            pub fn into_array_impl(self) -> ArrayImpl {
                let physical_type = self.0.physical_type();
                let boxed_any_array = self.0.into_any();
                match physical_type {
                    $(
                        PhysicalType::$Abc => ArrayImpl::$Abc(
                            *boxed_any_array.downcast::<$AbcArray>().expect("failed to downcast owned")
                        ),
                    )*
                }
            }

            /// Convert an [`BoxedArray`] into [`ArrayImpl`]
            pub fn as_array_impl(&self) -> ArrayImplRef<'_> {
                let physical_type = self.0.physical_type();
                let boxed_any_array: &dyn Any = self.0.as_any();
                match physical_type {
                    $(
                        PhysicalType::$Abc => ArrayImplRef::$Abc(
                            boxed_any_array.downcast_ref::<$AbcArray>().expect("failed to downcast ref")
                        ),
                    )*
                }
            }
        }
    };
}

for_all_variants! { impl_boxed_array_dispatch }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_boxed_array() {
        let a: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), Some(3), None]).into();
        let a = a.into_boxed_array();
        assert_eq!(a.get(0), Some(ScalarRefImpl::Int32(1)));
        let a = a.into_array_impl();
        assert_eq!(a.get(0), Some(ScalarRefImpl::Int32(1)));
    }
}
