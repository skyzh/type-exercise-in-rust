use crate::Array;

/// Iterates over nullable borrowed values from any typed array.
pub struct ArrayIterator<'a, A: Array> {
    array: &'a A,
    next_row: usize,
}

impl<'a, A: Array> ArrayIterator<'a, A> {
    pub fn new(array: &'a A) -> Self {
        Self { array, next_row: 0 }
    }
}

impl<'a, A: Array> Iterator for ArrayIterator<'a, A> {
    type Item = Option<A::RefItem<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_row == self.array.len() {
            return None;
        }

        let row = self.next_row;
        self.next_row += 1;
        let array: &'a A = self.array;
        Some(array.get(row))
    }
}
