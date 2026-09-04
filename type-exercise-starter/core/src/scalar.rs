/// Checkpoint 1: add the bounds connecting each owned scalar, borrowed scalar, and dense array.
pub trait Scalar {
    type ArrayType;
    // Rust requires this lifetime well-formedness clause for a GAT returned from `&self`; the
    // reciprocal family bounds are still learner work.
    type RefType<'a>
    where
        Self: 'a;
    fn as_scalar_ref(&self) -> Self::RefType<'_>;
}

/// Checkpoint 1: add the reciprocal owned, borrowed, and array relationships.
pub trait ScalarRef<'a> {
    type ArrayType;
    type ScalarType;
    fn to_owned_scalar(self) -> Self::ScalarType;
}

/// Checkpoint 1: extend this starter pair to every non-List physical family.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarImpl {
    Int32(i32),
    String(String),
    // Add the remaining primitive and Decimal variants.
    // A later checkpoint adds List.
}

/// Checkpoint 1: extend this borrowed inventory to match `ScalarImpl`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarRefImpl<'a> {
    Int32(i32),
    String(&'a str),
    // Add the remaining primitive and Decimal variants.
    // A later checkpoint adds List.
}

// Checkpoint 1: use the physical-family catalog to connect every scalar family to its array.
// Add physical-type methods and checked From/TryFrom conversions for both erased enums. Wrong
// variants return TypeMismatch (or a descriptive Decimal error) rather than panicking.

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
