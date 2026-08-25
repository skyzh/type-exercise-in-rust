use bitvec::vec::BitVec;

use crate::variant_catalog::for_each_physical_family;
use crate::{Array, ArrayBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    values: Vec<T>,
    validity: BitVec,
}

#[derive(Debug)]
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

// Day 10 adds null counting and `NonNullPrimitiveArray`.
