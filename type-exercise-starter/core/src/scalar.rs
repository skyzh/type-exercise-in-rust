/// Chapter 1: add the bounds that connect an owned scalar to its borrowed form. Keep
/// ArrayType as an unconstrained placeholder until the array step later in the chapter.
pub trait Scalar {
    type ArrayType;
    // Rust requires this lifetime well-formedness clause for a GAT returned from `&self`; the
    // reciprocal family bounds are still learner work in the first step.
    type RefType<'a>
    where
        Self: 'a;
    fn as_scalar_ref(&self) -> Self::RefType<'_>;
}

/// Chapter 1: add the reciprocal owned↔borrowed bounds. Keep ArrayType unconstrained
/// until the array step connects the concrete arrays.
pub trait ScalarRef<'a> {
    type ArrayType;
    type ScalarType;
    fn to_owned_scalar(self) -> Self::ScalarType;
}

/// The two owned scalar variants visible at the start of Chapter 1.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarImpl {
    Int32(i32),
    String(String),
    // Chapter 1: add the remaining primitive and Decimal variants.
    // Chapter 9: add `List(ListScalar)`.
}

/// The two borrowed scalar variants visible at the start of Chapter 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarRefImpl<'a> {
    Int32(i32),
    String(&'a str),
    // Chapter 1: add the remaining primitive and Decimal variants.
    // Chapter 9: add `List(ListScalarRef<'a>)`.
}

// Chapter 1: implement Scalar/ScalarRef for i32 and String/&str. The owned↔borrowed
// reciprocal bounds belong on the traits, not only on these concrete implementations.
// Chapter 1: connect each ArrayType placeholder to its concrete array.
// Chapter 1: add physical-type methods and checked From/TryFrom conversions for both
// erased enums. Wrong variants return TypeMismatch rather than panicking.
// Chapter 1: replace the repeated family relationships with catalog callbacks, then extend the
// catalog to the remaining scalar families and Decimal.

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
