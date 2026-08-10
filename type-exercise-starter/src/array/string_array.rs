use bitvec::vec::BitVec;

use crate::{Array, ArrayBuilder};

/// Day 1, checkpoint 2: implement shared UTF-8 bytes, offsets, and packed validity.
#[derive(Clone, Debug, PartialEq)]
pub struct StringArray;

#[derive(Clone, Debug, PartialEq)]
/// Day 1, checkpoint 2: implement append-only String construction.
pub struct StringArrayBuilder;

impl StringArray {
    pub fn data(&self) -> &[u8] {
        todo!("store shared UTF-8 bytes in Day 1")
    }
    pub fn offsets(&self) -> &[usize] {
        todo!("store monotone UTF-8 offsets in Day 1")
    }
    pub fn validity(&self) -> &BitVec {
        todo!("store packed string validity in Day 1")
    }
}

impl Array for StringArray {
    type Builder = StringArrayBuilder;
    type OwnedItem = String;
    type RefItem<'a> = &'a str;
    fn get(&self, _: usize) -> Option<Self::RefItem<'_>> {
        todo!("borrow a UTF-8 row in Day 1")
    }
    fn len(&self) -> usize {
        todo!("report string row count in Day 1")
    }
}

impl ArrayBuilder for StringArrayBuilder {
    type Array = StringArray;
    fn with_capacity(_: usize) -> Self {
        todo!("allocate string buffers in Day 1")
    }
    fn push(&mut self, _: Option<&str>) {
        todo!("append a string row in Day 1")
    }
    fn finish(self) -> Self::Array {
        todo!("finish a StringArray in Day 1")
    }
}
