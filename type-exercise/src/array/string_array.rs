use crate::{Array, ArrayBuilder};

/// A nullable UTF-8 array backed by bytes, offsets, and a validity vector.
#[derive(Clone, Debug, PartialEq)]
pub struct StringArray {
    data: Vec<u8>,
    offsets: Vec<usize>,
    valid: Vec<bool>,
}

/// The append-only builder for [`StringArray`].
#[derive(Debug)]
pub struct StringArrayBuilder {
    data: Vec<u8>,
    offsets: Vec<usize>,
    valid: Vec<bool>,
}

impl Array for StringArray {
    type Builder = StringArrayBuilder;
    type OwnedItem = String;
    type RefItem<'a> = &'a str;

    fn get(&self, row: usize) -> Option<Self::RefItem<'_>> {
        if !self.valid[row] {
            return None;
        }

        let bytes = &self.data[self.offsets[row]..self.offsets[row + 1]];
        Some(std::str::from_utf8(bytes).expect("StringArrayBuilder accepts only UTF-8 strings"))
    }

    fn len(&self) -> usize {
        self.valid.len()
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
            valid: Vec::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.data.extend_from_slice(value.as_bytes());
                self.valid.push(true);
            }
            None => self.valid.push(false),
        }
        self.offsets.push(self.data.len());
    }

    fn finish(self) -> Self::Array {
        StringArray {
            data: self.data,
            offsets: self.offsets,
            valid: self.valid,
        }
    }
}
