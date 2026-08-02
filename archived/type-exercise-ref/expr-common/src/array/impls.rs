// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

// Copyright 2022 RisingLight Project Authors. Licensed under Apache-2.0.

//! Contains all macro-generated implementations of array methods

use crate::array::all_array_builders::*;
use crate::array::all_arrays::*;
use crate::array::{Array, ArrayBuilder, ArrayBuilderImpl, ArrayImpl, ArrayImplRef, PhysicalType};
use crate::macros::for_all_variants;
use crate::scalar::*;
use crate::TypeMismatch;

/// Implements dispatch functions for [`Array`]
macro_rules! impl_array_dispatch {
    ([], $( { $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty } ),*) => {
        impl ArrayImpl {
            /// Create new [`ArrayBuilder`] from [`Array`] type.
            pub fn new_builder(&self, capacity: usize) -> ArrayBuilderImpl {
                match self {
                    $(
                        Self::$Abc(_) => ArrayBuilderImpl::$Abc(<$AbcArrayBuilder>::with_capacity(capacity))
                    ),*
                }
            }

            /// Get the value at the given index.
            pub fn get(&self, idx: usize) -> Option<ScalarRefImpl<'_>> {
                match self {
                    $(
                        Self::$Abc(array) => array.get(idx).map(ScalarRefImpl::$Abc),
                    )*
                }
            }

            /// Number of items of array.
            pub fn len(&self) -> usize {
                match self {
                    $(
                        Self::$Abc(a) => a.len(),
                    )*
                }
            }

            /// Number of items of array.
            pub fn is_empty(&self) -> bool {
                match self {
                    $(
                        Self::$Abc(a) => a.is_empty(),
                    )*
                }
            }

            /// Get physical type of the current array
            pub fn physical_type(&self) -> PhysicalType {
                match self {
                    $(
                        Self::$Abc(a) => a.physical_type(),
                    )*
                }
            }
        }
    }
}

for_all_variants! { impl_array_dispatch }

/// Implements dispatch functions for [`ArrayBuilder`]
macro_rules! impl_array_builder_dispatch {
    ([], $( { $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty } ),*) => {
        impl ArrayBuilderImpl {
            /// Appends an element to the back of array.
            pub fn push(&mut self, v: Option<ScalarRefImpl<'_>>) {
                match (self, v) {
                    $(
                        (Self::$Abc(a), Some(ScalarRefImpl::$Abc(v))) => a.push(Some(v)),
                        (Self::$Abc(a), None) => a.push(None),
                    )*
                    (a, Some(b)) => Err(TypeMismatch(a.physical_type(), b.physical_type())).unwrap(),
                }
            }

            /// Finish build and return a new array.
            pub fn finish(self) -> ArrayImpl {
                match self {
                    $(
                        Self::$Abc(a) => ArrayImpl::$Abc(a.finish()),
                    )*
                }
            }

            /// Get physical type of the current array builder
            pub fn physical_type(&self) -> PhysicalType {
                match self {
                    $(
                        Self::$Abc(a) => a.physical_type(),
                    )*
                }
            }
        }
    }
}

for_all_variants! { impl_array_builder_dispatch }

/// Implements `TryFrom` and `From` for [`Array`].
macro_rules! impl_array_conversion {
    ([], $({ $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty }),*) => {
        $(
            #[doc = concat!("Implement [`", stringify!($AbcArray), "`] -> [`ArrayImpl`]")]
            impl From<$AbcArray> for ArrayImpl {
                fn from(array: $AbcArray) -> Self {
                    Self::$Abc(array)
                }
            }

            #[doc = concat!("Implement [`ArrayImpl`] -> [`", stringify!($AbcArray), "`]")]
            impl TryFrom<ArrayImpl> for $AbcArray {
                type Error = TypeMismatch;

                fn try_from(array: ArrayImpl) -> Result<Self, Self::Error> {
                    match array {
                        ArrayImpl::$Abc(array) => Ok(array),
                        other => Err(TypeMismatch(PhysicalType::$Abc, other.physical_type())),
                    }
                }
            }

            #[doc = concat!("Implement reference of [`ArrayImpl`] -> [`", stringify!($AbcArray), "`]")]
            impl<'a> TryFrom<&'a ArrayImpl> for &'a $AbcArray {
                type Error = TypeMismatch;

                fn try_from(array: &'a ArrayImpl) -> Result<Self, Self::Error> {
                    match array {
                        ArrayImpl::$Abc(array) => Ok(array),
                        other => Err(TypeMismatch(PhysicalType::$Abc, other.physical_type())),
                    }
                }
            }

            #[doc = concat!("Implement [`", stringify!($AbcArrayBuilder), "`] -> [`ArrayBuilderImpl`]")]
            impl From<$AbcArrayBuilder> for ArrayBuilderImpl {
                fn from(builder: $AbcArrayBuilder) -> Self {
                    Self::$Abc(builder)
                }
            }

            #[doc = concat!("Implement [`ArrayBuilderImpl`] -> [`", stringify!($AbcArrayBuilder), "`]")]
            impl TryFrom<ArrayBuilderImpl> for $AbcArrayBuilder {
                type Error = TypeMismatch;

                fn try_from(builder: ArrayBuilderImpl) -> Result<Self, Self::Error> {
                    match builder {
                        ArrayBuilderImpl::$Abc(builder) => Ok(builder),
                        other => Err(TypeMismatch(PhysicalType::$Abc, other.physical_type())),
                    }
                }
            }

            #[doc = concat!("Implement mut ref of [`ArrayBuilderImpl`] -> [`", stringify!($AbcArrayBuilder), "`]")]
            impl<'a> TryFrom<&'a mut ArrayBuilderImpl> for &'a mut $AbcArrayBuilder {
                type Error = TypeMismatch;

                fn try_from(builder: &'a mut ArrayBuilderImpl) -> Result<Self, Self::Error> {
                    match builder {
                        ArrayBuilderImpl::$Abc(builder) => Ok(builder),
                        other => Err(TypeMismatch(PhysicalType::$Abc, other.physical_type())),
                    }
                }
            }
        )*

        impl ArrayImpl {
            /// Convert [`&ArrayImpl`] to [`ArrayImplRef`].
            pub fn as_ref(&self) -> ArrayImplRef<'_> {
                match self {
                    $(
                        ArrayImpl::$Abc(array) => ArrayImplRef::$Abc(array),
                    )*
                }
            }
        }
    };
}

for_all_variants! { impl_array_conversion }

fn debug_array<A: Array>(f: &mut std::fmt::Formatter<'_>, array: &A) -> std::fmt::Result {
    f.debug_list().entries(array.iter()).finish()
}

/// Implements Debug for [`Array`]
macro_rules! impl_array_debug {
    (
        [], $({ $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty }),*
    ) => {
        $(
            impl std::fmt::Debug for $AbcArray {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    debug_array(f, self)
                }
            }
        )*
    };
}

for_all_variants! { impl_array_debug }

/// Implements `physical_type` for [`Array`]
macro_rules! impl_physical_type {
    (
        [], $({ $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty }),*
    ) => {
        $(
            impl $AbcArray {
                fn physical_type(&self) -> PhysicalType {
                    PhysicalType::$Abc
                }
            }

            impl $AbcArrayBuilder {
                fn physical_type(&self) -> PhysicalType {
                    PhysicalType::$Abc
                }
            }
        )*
    };
}

for_all_variants! { impl_physical_type }
