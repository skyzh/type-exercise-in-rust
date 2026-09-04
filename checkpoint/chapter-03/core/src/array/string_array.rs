use bitvec::vec::BitVec;

use crate::{Array, ArrayBuilder};

/// A nullable UTF-8 array backed by one byte buffer and row offsets.
#[derive(Clone, Debug, PartialEq)]
pub struct StringArray {
    data: Vec<u8>,
    offsets: Vec<usize>,
    validity: BitVec,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringArrayBuilder {
    data: Vec<u8>,
    offsets: Vec<usize>,
    validity: BitVec,
}

impl StringArray {
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    pub fn validity(&self) -> &BitVec {
        &self.validity
    }
}

impl Array for StringArray {
    type Builder = StringArrayBuilder;
    type OwnedItem = String;
    type RefItem<'a> = &'a str;

    fn get(&self, row: usize) -> Option<Self::RefItem<'_>> {
        if !self.validity[row] {
            return None;
        }
        let bytes = &self.data[self.offsets[row]..self.offsets[row + 1]];
        Some(std::str::from_utf8(bytes).expect("StringArrayBuilder stores valid UTF-8"))
    }

    fn len(&self) -> usize {
        self.validity.len()
    }
}

impl ArrayBuilder for StringArrayBuilder {
    type Array = StringArray;

    fn with_capacity(capacity: usize) -> Self {
        let mut offsets = Vec::with_capacity(capacity + 1);
        offsets.push(0);
        Self {
            data: Vec::new(),
            offsets,
            validity: BitVec::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: Option<&str>) {
        if let Some(value) = value {
            self.data.extend_from_slice(value.as_bytes());
            self.validity.push(true);
        } else {
            self.validity.push(false);
        }
        self.offsets.push(self.data.len());
    }

    fn finish(self) -> Self::Array {
        StringArray {
            data: self.data,
            offsets: self.offsets,
            validity: self.validity,
        }
    }
}
