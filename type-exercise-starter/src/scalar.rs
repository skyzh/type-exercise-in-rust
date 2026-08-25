/// Day 1, checkpoint 1: add the bounds that connect an owned scalar to its borrowed and array
/// forms. The starter intentionally does not reveal those bounds.
pub trait Scalar {
    type ArrayType;
    // Rust requires this lifetime well-formedness clause for a GAT returned from `&self`; the
    // reciprocal family bounds are still learner work in checkpoint 1.
    type RefType<'a>
    where
        Self: 'a;
    fn as_scalar_ref(&self) -> Self::RefType<'_>;
}

/// Day 1, checkpoint 1: add the reciprocal bounds for a value borrowed from a scalar or array.
pub trait ScalarRef<'a> {
    type ArrayType;
    type ScalarType;
    fn to_owned_scalar(self) -> Self::ScalarType;
}

/// The two owned scalar variants visible at the start of Day 1.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarImpl {
    Int32(i32),
    String(String),
    // Day 2: add the remaining primitive and Decimal variants.
    // Day 11: add `List(ListScalar)`.
}

/// The two borrowed scalar variants visible at the start of Day 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarRefImpl<'a> {
    Int32(i32),
    String(&'a str),
    // Day 2: add the remaining primitive and Decimal variants.
    // Day 11: add `List(ListScalarRef<'a>)`.
}

// Day 1, checkpoint 1: implement Scalar/ScalarRef for i32 and String/&str. The reciprocal bounds
// belong on the traits, not only on these concrete implementations.
// Day 1, checkpoint 2: add physical-type methods and checked From/TryFrom conversions for both
// erased enums. Wrong variants return TypeMismatch rather than panicking.
// Day 2: extend those implementations to the remaining scalar families and Decimal.

#[cfg(test)]
mod tests {
    use super::ScalarImpl;

    #[test]
    fn starter_distinguishes_the_two_owned_scalar_variants() {
        assert_eq!(ScalarImpl::Int32(7), ScalarImpl::Int32(7));
        assert_eq!(
            ScalarImpl::String("rust".to_owned()),
            ScalarImpl::String("rust".to_owned())
        );
        assert_ne!(ScalarImpl::Int32(7), ScalarImpl::String("7".to_owned()));
    }
}
