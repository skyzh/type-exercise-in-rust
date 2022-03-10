// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

use std::fmt::Debug;
use std::ops::{Bound, RangeBounds};

use super::{Array, Scalar, ScalarRef, ScalarRefImpl};
use crate::array::{ArrayImplRef, BoxedArray, ListArray};
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

fn debug_array_ranged<A: Array>(
    f: &mut std::fmt::Formatter<'_>,
    array: &A,
    (from, to): (usize, usize),
) -> std::fmt::Result {
    f.debug_list()
        .entries(array.iter().skip(from).take(to - from))
        .finish()
}

/// Implements [`Debug`] trait for [`ListRef`]
macro_rules! impl_list_debug {
    (
        [], $({ $Abc:ident, $abc:ident, $AbcArray:ty, $AbcArrayBuilder:ty, $Owned:ty, $Ref:ty }),*
    ) => {
        impl<'a> Debug for ListRef<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.array.as_array_impl() {
                    $(
                        ArrayImplRef::$Abc(array) => debug_array_ranged(f, array, self.offset),
                    )*
                }
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

    fn slice_from_to(&self, from: usize, to: usize) -> Self {
        assert!(to <= self.offset.1);
        assert!(from >= self.offset.0);
        Self {
            array: self.array,
            offset: (from, to),
        }
    }

    pub fn slice(&self, range: impl RangeBounds<usize>) -> Self {
        let (l, r) = self.offset;
        let ll = match range.start_bound() {
            Bound::Unbounded => l,
            Bound::Included(x) => l + x,
            Bound::Excluded(x) => l + x + 1,
        };
        let rr = match range.end_bound() {
            Bound::Unbounded => r,
            Bound::Included(x) => l + x + 1,
            Bound::Excluded(x) => l + x,
        };
        self.slice_from_to(ll, rr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::{Array, ArrayImpl, I32Array};

    #[test]
    fn test_list_debug() {
        let x: ArrayImpl = I32Array::from_slice(&[Some(0), Some(1), None]).into();
        let x = x.into_boxed_array();
        let list_ref: ListRef = (&x).into();
        assert_eq!(format!("{:?}", list_ref), "[Some(0), Some(1), None]");
        assert_eq!(
            format!("{:?}", list_ref.slice(..)),
            "[Some(0), Some(1), None]"
        );
        assert_eq!(format!("{:?}", list_ref.slice(1..)), "[Some(1), None]");
        assert_eq!(format!("{:?}", list_ref.slice(..1)), "[Some(0)]");
        assert_eq!(format!("{:?}", list_ref.slice(1..2)), "[Some(1)]");
        assert_eq!(format!("{:?}", list_ref.slice(..=0)), "[Some(0)]");
        assert_eq!(format!("{:?}", list_ref.slice(1..=2)), "[Some(1), None]");
    }
}
