// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

use bitvec::prelude::BitVec;

use super::{Array, ArrayBuilder, ArrayBuilderImpl, ArrayIterator, BoxedArray};
use crate::scalar::{List, ListRef};

#[derive(Clone)]
pub struct ListArray {
    /// The actual data of this array.
    data: BoxedArray,

    /// The offsets of each list element
    offsets: Vec<usize>,

    /// The null bitmap of this array.
    bitmap: BitVec,
}

impl Array for ListArray {
    type Builder = ListArrayBuilder;

    type OwnedItem = List;

    type RefItem<'a> = ListRef<'a>;

    fn get(&self, idx: usize) -> Option<ListRef<'_>> {
        if self.bitmap[idx] {
            Some(ListRef {
                array: &self.data,
                offset: (self.offsets[idx], self.offsets[idx + 1]),
            })
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn iter(&self) -> ArrayIterator<Self> {
        ArrayIterator::new(self)
    }
}

pub struct ListArrayBuilder {
    /// The actual data of this array.
    builder: Box<Option<ArrayBuilderImpl>>,

    /// The offsets of each list element
    offsets: Vec<usize>,

    /// The null bitmap of this array.
    bitmap: BitVec,

    /// Number of items in this array
    number_of_items: usize,
}

impl ArrayBuilder for ListArrayBuilder {
    type Array = ListArray;

    fn with_capacity(capacity: usize) -> Self {
        let mut offsets = Vec::with_capacity(capacity + 1);
        offsets.push(0);
        Self {
            builder: Box::new(None),
            bitmap: BitVec::with_capacity(capacity),
            offsets,
            number_of_items: 0,
        }
    }

    fn push(&mut self, value: Option<ListRef<'_>>) {
        match value {
            Some(v) => {
                // Dynamically detect the `ListArray` type upon first push.
                if self.builder.is_none() {
                    self.builder = Box::new(Some(v.array.new_builder(self.bitmap.capacity())));
                }
                let builder = (*self.builder).as_mut().unwrap();
                for i in v.offset.0..v.offset.1 {
                    builder.push(v.array.get(i));
                }
                self.bitmap.push(true);
                self.number_of_items += v.len();
                self.offsets.push(self.number_of_items);
            }
            None => {
                self.offsets.push(self.number_of_items);
                self.bitmap.push(false);
            }
        }
    }

    fn finish(self) -> Self::Array {
        ListArray {
            data: self
                .builder
                .expect("cannot create an empty list array")
                .finish()
                .into_boxed_array(),
            bitmap: self.bitmap,
            offsets: self.offsets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ListArrayBuilder;
    use crate::array::*;
    use crate::scalar::ScalarRefImpl;

    #[test]
    fn test_list_build() {
        let mut builder = ListArrayBuilder::with_capacity(0);
        let array1: ArrayImpl = I32Array::from_slice(&[Some(0), Some(1), Some(2)]).into();
        let array1 = array1.into_boxed_array();
        builder.push(Some((&array1).into()));
        let array2: ArrayImpl = I32Array::from_slice(&[]).into();
        let array2 = array2.into_boxed_array();
        builder.push(Some((&array2).into()));
        builder.push(None);
        let array3: ArrayImpl = I32Array::from_slice(&[Some(0), None, Some(2)]).into();
        let array3 = array3.into_boxed_array();
        builder.push(Some((&array3).into()));
        let list_array = builder.finish();

        let array1 = list_array.get(0).unwrap();
        assert_eq!(array1.len(), 3);
        assert_eq!(array1.get(0), Some(ScalarRefImpl::Int32(0)));
        assert_eq!(array1.get(1), Some(ScalarRefImpl::Int32(1)));
        assert_eq!(array1.get(2), Some(ScalarRefImpl::Int32(2)));

        let array2 = list_array.get(1).unwrap();
        assert_eq!(array2.len(), 0);

        let array3 = list_array.get(2);
        assert!(array3.is_none());

        let array4 = list_array.get(3).unwrap();
        assert_eq!(array4.len(), 3);
        assert_eq!(array4.get(0), Some(ScalarRefImpl::Int32(0)));
        assert_eq!(array4.get(1), None);
        assert_eq!(array4.get(2), Some(ScalarRefImpl::Int32(2)));
    }
}
