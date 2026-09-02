use crate::{Scalar, ScalarRef};

// === Chapter 1 checkpoint 1 ===

/// This generic body must compile from `S: Scalar` alone. It proves that every
/// `S::RefType<'a>` points back to exactly `S`, not merely to some scalar type.
#[allow(dead_code)]
fn borrowed_value_returns_to_its_scalar<S: Scalar>(value: &S) -> S {
    value.as_scalar_ref().to_owned_scalar()
}

#[test]
fn checkpoint_1_connects_scalar_and_scalar_ref() {
    let integer = 42_i32;
    let integer_ref: <i32 as Scalar>::RefType<'_> = integer.as_scalar_ref();
    assert_eq!(integer_ref, 42);
    assert_eq!(integer_ref.to_owned_scalar(), 42);

    let string = String::from("type system");
    let string_ref: <String as Scalar>::RefType<'_> = string.as_scalar_ref();
    assert_eq!(string_ref, "type system");
    assert_eq!(string_ref.to_owned_scalar(), string);
}

// === Chapter 1 checkpoint 2 ===
