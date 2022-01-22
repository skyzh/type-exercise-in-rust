// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

// Copyright 2022 RisingLight Project Authors. Licensed under Apache-2.0.

//! Contains all macro-generated implementations of scalar methods

use crate::array::*;
use crate::macros::{for_all_primitive_variants, for_all_variants};
use crate::scalar::*;
use crate::TypeMismatch;

/// Implements dispatch functions for [`Scalar`]
macro_rules! impl_scalar_dispatch {
    ([], $( { $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty } ),*) => {
        impl ScalarImpl {
            /// Get identifier of the current array
            pub fn identifier(&self) -> &'static str {
                match self {
                    $(
                        Self::$Abc(_) => stringify!($Abc),
                    )*
                }
            }
        }
    }
}

for_all_variants! { impl_scalar_dispatch }

/// Implements dispatch functions for [`ScalarRef`]
macro_rules! impl_scalar_ref_dispatch {
    ([], $( { $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty } ),*) => {
        impl <'a> ScalarRefImpl<'a> {
            /// Get identifier of the current array
            pub fn identifier(&self) -> &'static str {
                match self {
                    $(
                        Self::$Abc(_) => stringify!($Abc),
                    )*
                }
            }
        }
    }
}

for_all_variants! { impl_scalar_ref_dispatch }

/// Implements `TryFrom` and `From` for [`Scalar`] and [`ScalarRef`].
macro_rules! impl_scalar_conversion {
    ([], $({ $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty }),*) => {
        $(
            #[doc = concat!(
                "Implement [`ScalarImpl`] -> [`", stringify!($Owned), "`], return [`TypeMismatch`] error if mismatch")]
            impl<'a> TryFrom<ScalarImpl> for $Owned {
                type Error = TypeMismatch;
                fn try_from(that: ScalarImpl) -> Result<Self, Self::Error> {
                    match that {
                        ScalarImpl::$Abc(v) => Ok(v),
                        other => Err(TypeMismatch(stringify!($Abc), other.identifier())),
                    }
                }
            }

            #[doc = concat!("Implement [`", stringify!($Owned), "`] -> [`ScalarImpl`]")]
            impl From<$Owned> for ScalarImpl {
                fn from(that: $Owned) -> Self {
                    ScalarImpl::$Abc(that)
                }
            }

            #[doc = concat!(
                "Implement [`ScalarRefImpl`] -> [`", stringify!($Ref), "`], return [`TypeMismatch`] error if mismatch")]
            impl<'a> TryFrom<ScalarRefImpl<'a>> for $Ref {
                type Error = TypeMismatch;
                fn try_from(that: ScalarRefImpl<'a>) -> Result<Self, Self::Error> {
                    match that {
                        ScalarRefImpl::$Abc(v) => Ok(v),
                        other => Err(TypeMismatch(stringify!($Abc), other.identifier())),
                    }
                }
            }

            #[doc = concat!("Implement [`", stringify!($Ref), "`] -> [`ScalarRefImpl`]")]
            impl<'a> From<$Ref> for ScalarRefImpl<'a> {
                fn from(that: $Ref) -> Self {
                    ScalarRefImpl::$Abc(that)
                }
            }
        )*
    };
}

for_all_variants! { impl_scalar_conversion }

/// Implements [`Scalar`] trait for primitive types
macro_rules! impl_scalar {
    ([], $( { $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty } ),*) => {
        $(
            #[doc = concat!(
                "Implement [`Scalar`] for primitive type [`", stringify!($Owned), "`]. ",
                "Note that primitive types are both [`Scalar`] and [`ScalarRef`] as they have little cost for copy.")]
            impl Scalar for $Owned {
                type ArrayType = $AbcArray;
                type RefType<'a> = $Owned;

                fn as_scalar_ref(&self) -> $Owned {
                    *self
                }
            }

            #[doc = concat!(
                "Implement [`ScalarRef`] for primitive type [`", stringify!($Ref), "`]. ",
                "Note that primitive types are both [`Scalar`] and [`ScalarRef`] as they have little cost for copy.")]
            impl<'a> ScalarRef<'a> for $Owned {
                type ArrayType = $AbcArray;
                type ScalarType = $Owned;

                fn to_owned_scalar(&self) -> $Owned {
                    *self
                }
            }
        )*
    }
}

for_all_primitive_variants! { impl_scalar }

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
