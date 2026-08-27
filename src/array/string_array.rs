use crate::{Array, ArrayBuilder};
use bitvec::vec::BitVec;

/// A nullable UTF-8 array backed by bytes, offsets, and a packed validity bitmap.
#[derive(Clone, Debug, PartialEq)]
pub struct StringArray {
    data: Vec<u8>,
    offsets: Vec<usize>,
    validity: BitVec,
}

/// The append-only builder for [`StringArray`].
#[derive(Debug)]
pub struct StringArrayBuilder {
    data: Vec<u8>,
    offsets: Vec<usize>,
    validity: BitVec,
}

impl StringArray {
    /// The contiguous UTF-8 byte buffer.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// The row boundaries into [`Self::data`].
    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    /// The packed row-validity bitmap.
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
        Some(std::str::from_utf8(bytes).expect("StringArrayBuilder accepts only UTF-8 strings"))
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
        match value {
            Some(value) => {
                self.data.extend_from_slice(value.as_bytes());
                self.validity.push(true);
            }
            None => self.validity.push(false),
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

#[cfg(test)]
mod tests {
    use bitvec::vec::BitVec;

    use crate::{Array, StringArray};

    #[test]
    fn strings_share_one_byte_buffer_with_offsets_and_packed_validity() {
        let array = StringArray::from_slice(&[Some("a"), None, Some("é"), Some("")]);
        let validity: &BitVec = array.validity();

        assert_eq!(array.data(), "aé".as_bytes());
        assert_eq!(array.offsets(), &[0, 1, 1, 3, 3]);
        assert_eq!(
            validity.iter().by_vals().collect::<Vec<_>>(),
            [true, false, true, true]
        );
        assert_eq!(array.offsets().len(), validity.len() + 1);
        assert!(array.offsets().windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(array.offsets().last().copied(), Some(array.data().len()));

        let borrowed = array.get(2).unwrap();
        assert_eq!(
            borrowed.as_ptr() as usize,
            array.data().as_ptr() as usize + array.offsets()[2]
        );
    }

    #[test]
    fn empty_and_all_null_strings_keep_valid_arrow_like_offsets() {
        let empty = StringArray::from_slice(&[]);
        assert!(empty.data().is_empty());
        assert_eq!(empty.offsets(), &[0]);
        assert!(empty.validity().is_empty());

        let all_null = StringArray::from_slice(&[None, None, None]);
        assert!(all_null.data().is_empty());
        assert_eq!(all_null.offsets(), &[0, 0, 0, 0]);
        assert_eq!(
            all_null.validity().iter().by_vals().collect::<Vec<_>>(),
            [false, false, false]
        );
    }
}
