// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Implements `Arrayiterator`

use std::iter::TrustedLen;

use super::Array;

/// An iterator that iterators on any [`Array`] type.
pub struct ArrayIterator<'a, A: Array> {
    array: &'a A,
    pos: usize,
}

impl<'a, A: Array> Iterator for ArrayIterator<'a, A> {
    type Item = Option<A::RefItem<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.array.len() {
            None
        } else {
            let item = self.array.get(self.pos);
            self.pos += 1;
            Some(item)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.array.len() - self.pos,
            Some(self.array.len() - self.pos),
        )
    }
}

impl<'a, A: Array> ArrayIterator<'a, A> {
    /// Create an [`ArrayIterator`] from [`Array`].
    pub fn new(array: &'a A) -> Self {
        Self { array, pos: 0 }
    }
}

impl<'a, A: Array> ExactSizeIterator for ArrayIterator<'a, A> {
    fn len(&self) -> usize {
        self.array.len() - self.pos
    }
}

unsafe impl<'a, A: Array> TrustedLen for ArrayIterator<'a, A> {}
