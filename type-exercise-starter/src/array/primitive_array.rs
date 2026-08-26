use std::marker::PhantomData;

/// Day 1, checkpoint 3: replace the marker with flat Int32 values plus packed validity.
/// Day 2, checkpoint 1: generalize the same layout to the remaining primitive families.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveArray<T> {
    marker: PhantomData<T>,
}

#[derive(Clone, Debug, PartialEq)]
/// Day 1, checkpoint 3: replace the marker with append-only value and validity buffers.
pub struct PrimitiveArrayBuilder<T> {
    marker: PhantomData<T>,
}

pub type I32Array = PrimitiveArray<i32>;
pub type I32ArrayBuilder = PrimitiveArrayBuilder<i32>;

// Day 1, checkpoint 3: add `values`/`validity`, implement Array for I32Array, and implement its
// builder. Day 2 generalizes the implementation and adds I16, I64, Bool, F32, and F64 aliases.
// Day 10 keeps this one representation and lets a checked `ColumnViewImpl` carry physical
// `Nullability`; it does not add a second primitive array type or cache a null count.
