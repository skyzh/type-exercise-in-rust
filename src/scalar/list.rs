use std::fmt::Debug;

use super::{Scalar, ScalarRef, ScalarRefImpl};
use crate::array::{Array, BoxedArray, ListArray};
use crate::macros::for_all_variants;

#[derive(Clone, Debug)]
pub struct List(BoxedArray);

#[derive(Clone, Copy)]
pub struct ListRef<'a> {
    pub(crate) array: &'a BoxedArray,
    pub(crate) offset: (usize, usize),
}

impl<'a> From<&'a BoxedArray> for ListRef<'a> {
    fn from(array: &'a BoxedArray) -> Self {
        Self {
            array,
            offset: (0, array.len()),
        }
    }
}

/// Implements [`Debug`] trait for [`ListRef`]
macro_rules! impl_list_debug {
    (
        [], $({ $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty }),*
    ) => {
        impl<'a> Debug for ListRef<'a> {
            fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                todo!()
            }
        }
    };
}

for_all_variants! { impl_list_debug }

/// Implement [`Scalar`] for `List`.
impl Scalar for List {
    type ArrayType = ListArray;
    type RefType<'a> = ListRef<'a>;

    fn as_scalar_ref(&self) -> ListRef<'_> {
        ListRef {
            array: &self.0,
            offset: (0, self.0.len()),
        }
    }

    #[allow(clippy::needless_lifetimes)]
    fn cast_s_to_a<'x>(item: Self::RefType<'x>) -> <Self::ArrayType as Array>::RefItem<'x> {
        item
    }

    #[allow(clippy::needless_lifetimes)]
    fn cast_a_to_s<'x>(item: <Self::ArrayType as Array>::RefItem<'x>) -> Self::RefType<'x> {
        item
    }

    fn upcast_gat<'short, 'long: 'short>(long: ListRef<'long>) -> ListRef<'short> {
        long
    }
}

impl List {
    /// Get length of [`List`]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, idx: usize) -> Option<ScalarRefImpl<'_>> {
        self.0.get(idx)
    }
}

/// Implement [`ScalarRef`] for `ListRef<'a>`.
impl<'a> ScalarRef<'a> for ListRef<'a> {
    type ArrayType = ListArray;
    type ScalarType = List;

    fn to_owned_scalar(&self) -> List {
        let mut builder = self.array.new_builder(self.len());
        for idx in self.offset.0..self.offset.1 {
            builder.push(self.array.get(idx));
        }
        List(builder.finish().into_boxed_array())
    }
}

impl<'a> ListRef<'a> {
    /// Get length of [`List`]
    pub fn len(&self) -> usize {
        self.offset.1 - self.offset.0
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, idx: usize) -> Option<ScalarRefImpl<'_>> {
        assert!(
            idx + self.offset.0 < self.offset.1,
            "out of bound when accessing ListRef"
        );
        self.array.get(idx + self.offset.0)
    }
}
