use std::marker::PhantomData;

/// Chapter 1: replace the marker with flat Int32 values plus packed validity.
/// Chapter 1: generalize the same layout to the remaining primitive families.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    marker: PhantomData<T>,
}

#[derive(Clone, Debug, PartialEq)]
/// Chapter 1: replace the marker with append-only value and validity buffers.
pub struct PrimitiveArrayBuilder<T> {
    marker: PhantomData<T>,
}

pub type I32Array = PrimitiveArray<i32>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;

// Chapter 1: add `values`/`validity`, implement Array for I32Array, and implement its
// builder. Chapter 1 adds six explicit aliases and one generic Array/ArrayBuilder implementation
// instead of generating identical implementations with macros.
// Chapter 6 keeps this one representation and lets `ColumnViewImpl` borrow its raw values and
// validity privately; it does not add a second primitive array type or cache a null count.
