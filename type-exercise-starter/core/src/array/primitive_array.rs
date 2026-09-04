use std::marker::PhantomData;

/// Checkpoint 1: replace the marker with flat values plus packed validity.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    marker: PhantomData<T>,
}

#[derive(Clone, Debug, PartialEq)]
/// Checkpoint 1: replace the marker with append-only value and validity buffers.
pub struct PrimitiveArrayBuilder<T> {
    marker: PhantomData<T>,
}

pub type I32Array = PrimitiveArray<i32>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;

// Checkpoint 1: add aliases for all six fixed-width families, expose read-only values/validity,
// and implement one generic Array/ArrayBuilder pair.
