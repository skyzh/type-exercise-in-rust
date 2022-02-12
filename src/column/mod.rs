use crate::array::{Array, ArrayIterator};
use crate::scalar::Scalar;

/// An array-like structure that can be used to represent a column of data.
pub trait Column<'a> {
    type ColumnIter: Iterator<Item = Option<<Self::ScalarType as Scalar>::RefType<'a>>>;
    type ScalarType: Scalar;

    /// Get an iterator to the column
    fn iter(&self) -> Self::ColumnIter;

    /// Get the length of the column
    fn len(&self) -> usize;
}

impl<'a, A: Array> Column<'a> for &'a A {
    type ColumnIter = ArrayIterator<'a, A>;
    type ScalarType = A::OwnedItem;

    fn iter(&self) -> Self::ColumnIter {
        ArrayIterator::new(self)
    }

    fn len(&self) -> usize {
        Array::len(*self)
    }
}

pub struct ConstantArray<S: Scalar> {
    value: Option<S>,
    len: usize,
}

impl<'a, S: Scalar> Column<'a> for &'a ConstantArray<S> {
    type ColumnIter = ConstantArrayIterator<'a, S>;
    type ScalarType = S;

    fn iter(&self) -> Self::ColumnIter {
        ConstantArrayIterator::new(self)
    }

    fn len(&self) -> usize {
        self.len
    }
}

pub struct ConstantArrayIterator<'a, S: Scalar> {
    value: Option<S::RefType<'a>>,
    len: usize,
    idx: usize,
}

impl<'a, S: Scalar> ConstantArrayIterator<'a, S> {
    pub fn new(column: &'a ConstantArray<S>) -> Self {
        ConstantArrayIterator {
            value: column.value.as_ref().map(|v| v.as_scalar_ref()),
            len: column.len,
            idx: 0,
        }
    }
}

impl<'a, S: Scalar> Iterator for ConstantArrayIterator<'a, S> {
    type Item = Option<S::RefType<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            None
        } else {
            self.idx += 1;
            Some(self.value)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Add;

    use super::*;
    use crate::array::{ArrayBuilder, I64Array};
    use crate::scalar::ScalarRef;

    fn vectorized_add<'l, 'r, L, R, LC, RC, O>(i1: LC, i2: RC) -> O::ArrayType
    where
        L: Scalar,
        R: Scalar,
        O: Scalar,
        LC: Column<'l, ScalarType = L>,
        RC: Column<'r, ScalarType = R>,
        L: Into<O>,
        R: Into<O>,
        O: Add<Output = O>,
    {
        let mut output = <O::ArrayType as Array>::Builder::with_capacity(0);
        for (i1, i2) in i1.iter().zip(i2.iter()) {
            match (i1, i2) {
                (Some(i1), Some(i2)) => {
                    let i1: L = i1.to_owned_scalar();
                    let i2: R = i2.to_owned_scalar();
                    let i1: O = i1.into();
                    let i2: O = i2.into();
                    let o: O = i1 + i2;
                    output.push(Some(o.as_scalar_ref()));
                }
                _ => output.push(None),
            }
        }
        output.finish()
    }

    #[test]
    fn test_constant_array_iterator() {
        let a1 = ConstantArray {
            value: Some(1 as i64),
            len: 3,
        };
        let a2 = I64Array::from_slice(&[Some(1), Some(2), Some(3)]);
        vectorized_add::<_, _, _, _, i64>(&a1, &a2);
    }
}
