use crate::variant_catalog::for_each_physical_family;
use crate::{Array, ArrayBuilder, Decimal};

/// A compact teaching representation for nullable fixed-width values.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    values: Vec<T>,
    validity: Vec<bool>,
    null_count: usize,
}

/// A checked view proving that every slot in a primitive array is valid.
#[derive(Clone, Copy, Debug)]
pub struct NonNullPrimitiveArray<'a, T> {
    array: &'a PrimitiveArray<T>,
}

/// The append-only builder for [`PrimitiveArray`].
#[derive(Debug)]
pub struct PrimitiveArrayBuilder<T> {
    values: Vec<T>,
    validity: Vec<bool>,
    null_count: usize,
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
}

for_each_physical_family!(define_primitive_aliases);

impl<T> PrimitiveArray<T> {
    pub fn from_values(values: Vec<T>) -> Self {
        Self {
            validity: vec![true; values.len()],
            values,
            null_count: 0,
        }
    }

    pub fn null_count(&self) -> usize {
        self.null_count
    }

    pub fn as_non_null(&self) -> Option<NonNullPrimitiveArray<'_, T>> {
        (self.null_count == 0).then_some(NonNullPrimitiveArray { array: self })
    }
}

impl<'a, T> NonNullPrimitiveArray<'a, T> {
    pub fn values(self) -> &'a [T] {
        &self.array.values
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
                    validity: Vec::with_capacity(capacity),
                    null_count: 0,
                }
            }

            fn push(&mut self, value: Option<$owned>) {
                match value {
                    Some(value) => {
                        self.values.push(value);
                        self.validity.push(true);
                    }
                    None => {
                        self.values.push(<$owned>::default());
                        self.validity.push(false);
                        self.null_count += 1;
                    }
                }
            }

            fn finish(self) -> Self::Array {
                PrimitiveArray {
                    values: self.values,
                    validity: self.validity,
                    null_count: self.null_count,
                }
            }
        }
    };
    (borrowed, $array:ident, $builder:ident, $owned:ty) => {};
}

for_each_physical_family!(implement_primitive_families);
