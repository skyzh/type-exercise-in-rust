use bitvec::vec::BitVec;

use crate::variant_catalog::for_each_physical_family;
use crate::{Array, ArrayBuilder};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveArray<T> {
    values: Vec<T>,
    validity: BitVec,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveArrayBuilder<T> {
    values: Vec<T>,
    validity: BitVec,
}

macro_rules! define_primitive_aliases {
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),+ $(,)?) => {
        $(define_primitive_alias!($kind, $array, $builder, $owned);)+
    };
}

macro_rules! define_primitive_alias {
    (copy, $array:ident, $builder:ident, $owned:ty) => {
        pub type $array = PrimitiveArray<$owned>;
        pub type $builder = PrimitiveArrayBuilder<$owned>;
    };
    (borrowed, $array:ident, $builder:ident, $owned:ty) => {};
    (decimal, $array:ident, $builder:ident, $owned:ty) => {};
}

for_each_physical_family!(define_primitive_aliases);

impl<T> PrimitiveArray<T> {
    pub fn from_values(values: Vec<T>) -> Self {
        Self {
            validity: bitvec::bitvec![1; values.len()],
            values,
        }
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn validity(&self) -> &BitVec {
        &self.validity
    }

    pub(crate) fn from_raw_parts(values: Vec<T>, validity: BitVec) -> Self {
        debug_assert_eq!(values.len(), validity.len());
        Self { values, validity }
    }

    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
}

impl<T> PrimitiveArrayBuilder<T> {
    pub(crate) fn with_raw_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            validity: BitVec::with_capacity(capacity),
        }
    }

    pub(crate) fn push_raw(&mut self, value: T, valid: bool) {
        self.values.push(value);
        self.validity.push(valid);
    }

    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn finish_raw(self) -> PrimitiveArray<T> {
        PrimitiveArray::from_raw_parts(self.values, self.validity)
    }
}

macro_rules! implement_primitive_families {
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),+ $(,)?) => {
        $(implement_primitive_family!($kind, $array, $builder, $owned);)+
    };
}

macro_rules! implement_primitive_family {
    (copy, $array:ident, $builder:ident, $owned:ty) => {
        impl Array for $array {
            type Builder = $builder;
            type OwnedItem = $owned;
            type RefItem<'a> = $owned;

            fn get(&self, row: usize) -> Option<Self::RefItem<'_>> {
                self.validity[row].then_some(self.values[row])
            }

            fn len(&self) -> usize {
                self.values.len()
            }
        }

        impl ArrayBuilder for $builder {
            type Array = $array;

            fn with_capacity(capacity: usize) -> Self {
                Self {
                    values: Vec::with_capacity(capacity),
                    validity: BitVec::with_capacity(capacity),
                }
            }

            fn push(&mut self, value: Option<$owned>) {
                self.values.push(value.unwrap_or_default());
                self.validity.push(value.is_some());
            }

            fn finish(self) -> Self::Array {
                PrimitiveArray {
                    values: self.values,
                    validity: self.validity,
                }
            }
        }
    };
    (borrowed, $array:ident, $builder:ident, $owned:ty) => {};
    (decimal, $array:ident, $builder:ident, $owned:ty) => {};
}

for_each_physical_family!(implement_primitive_families);

// Day 10 keeps this one representation and moves the checked non-null proof to column metadata.
