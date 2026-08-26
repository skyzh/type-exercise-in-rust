use crate::variant_catalog::for_each_physical_family;
use crate::{Array, ArrayBuilder};
use bitvec::vec::BitVec;

/// A compact teaching representation for nullable fixed-width values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveArray<T> {
    values: Vec<T>,
    validity: BitVec,
}

/// The append-only builder for [`PrimitiveArray`].
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
            validity: BitVec::repeat(true, values.len()),
            values,
        }
    }

    pub(crate) fn from_raw_parts(values: Vec<T>, validity: BitVec) -> Self {
        debug_assert_eq!(values.len(), validity.len());
        Self { values, validity }
    }

    /// The contiguous fixed-width value buffer.
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// The packed row-validity bitmap.
    pub fn validity(&self) -> &BitVec {
        &self.validity
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
                match value {
                    Some(value) => {
                        self.values.push(value);
                        self.validity.push(true);
                    }
                    None => {
                        self.values.push(<$owned>::default());
                        self.validity.push(false);
                    }
                }
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

#[cfg(test)]
mod tests {
    use bitvec::vec::BitVec;

    use super::PrimitiveArray;
    use crate::{Array, I32Array};

    #[test]
    fn fixed_width_values_and_validity_have_arrow_like_storage() {
        let array = I32Array::from_slice(&[Some(10), None, Some(30)]);
        let validity: &BitVec = array.validity();

        assert_eq!(array.values(), &[10, 0, 30]);
        assert_eq!(
            validity.iter().by_vals().collect::<Vec<_>>(),
            [true, false, true]
        );
        assert_eq!(array.values().len(), validity.len());

        let many = PrimitiveArray::from_values((0_i32..130).collect());
        assert!(std::mem::size_of_val(many.validity().as_raw_slice()) < many.len());
    }
}
