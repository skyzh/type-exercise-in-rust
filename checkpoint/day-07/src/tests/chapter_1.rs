use bitvec::vec::BitVec;

use crate::{
    Array, ArrayBuilder, ArrayImpl, I32Array, PHYSICAL_FAMILY_CATALOG, PhysicalFamily, Scalar,
    ScalarImpl, ScalarRef, ScalarRefImpl, StringArray, StringArrayBuilder,
};

/// A small stand-in for a database comparison expression. Its body compiles with only `S:
/// Scalar` because the reciprocal bounds live on the type-family traits rather than at every call
/// site.
#[allow(dead_code)]
fn rows_are_equal<S: Scalar>(array: &S::ArrayType, left: usize, right: usize) -> Option<bool>
where
    for<'a> S::RefType<'a>: PartialEq,
{
    Some(array.get(left)? == array.get(right)?)
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

#[test]
fn checkpoint_2_erases_and_recovers_scalars() {
    assert_eq!(i32::try_from(ScalarImpl::from(9_i32)).unwrap(), 9);
    assert_eq!(
        String::try_from(ScalarImpl::from(String::from("owned"))).unwrap(),
        "owned"
    );
    assert_eq!(i32::try_from(ScalarRefImpl::from(7_i32)).unwrap(), 7);
    assert_eq!(
        <&str>::try_from(ScalarRefImpl::from("borrowed")).unwrap(),
        "borrowed"
    );

    assert!(i32::try_from(ScalarImpl::String("wrong".to_owned())).is_err());
    assert!(<&str>::try_from(ScalarRefImpl::Int32(1)).is_err());
}

#[test]
fn checkpoint_3_builds_arrow_like_arrays() {
    let integers = I32Array::from_slice(&[Some(10), None, Some(30)]);
    assert_eq!(integers.len(), 3);
    assert_eq!(
        integers.iter().collect::<Vec<_>>(),
        vec![Some(10), None, Some(30)]
    );

    let integer_validity: &BitVec = integers.validity();
    assert_eq!(integers.values(), &[10, 0, 30]);
    assert_eq!(
        integer_validity.iter().by_vals().collect::<Vec<_>>(),
        [true, false, true]
    );
    assert_eq!(integers.values().len(), integer_validity.len());

    let packed = I32Array::from_slice(&vec![Some(1); 130]);
    assert!(std::mem::size_of_val(packed.validity().as_raw_slice()) < packed.len());

    let strings = StringArray::from_slice(&[Some("a"), None, Some("é"), Some("")]);
    let string_validity: &BitVec = strings.validity();
    assert_eq!(strings.data(), "aé".as_bytes());
    assert_eq!(strings.offsets(), &[0, 1, 1, 3, 3]);
    assert_eq!(
        string_validity.iter().by_vals().collect::<Vec<_>>(),
        [true, false, true, true]
    );
    assert_eq!(strings.offsets().len(), string_validity.len() + 1);
    assert!(strings.offsets().windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(
        strings.offsets().last().copied(),
        Some(strings.data().len())
    );

    let borrowed = strings.get(2).unwrap();
    assert_eq!(
        borrowed.as_ptr() as usize,
        strings.data().as_ptr() as usize + strings.offsets()[2]
    );

    let source = String::from("owned by the array");
    let mut builder = StringArrayBuilder::with_capacity(1);
    builder.push(Some(source.as_str()));
    drop(source);
    assert_eq!(builder.finish().get(0), Some("owned by the array"));

    let empty_integers = I32Array::from_slice(&[]);
    assert!(empty_integers.values().is_empty());
    assert!(empty_integers.validity().is_empty());

    let null_integers = I32Array::from_slice(&[None, None, None]);
    assert_eq!(null_integers.values(), &[0, 0, 0]);
    assert_eq!(
        null_integers
            .validity()
            .iter()
            .by_vals()
            .collect::<Vec<_>>(),
        [false, false, false]
    );

    let empty_strings = StringArray::from_slice(&[]);
    assert!(empty_strings.data().is_empty());
    assert_eq!(empty_strings.offsets(), &[0]);
    assert!(empty_strings.validity().is_empty());

    let null_strings = StringArray::from_slice(&[None, None, None]);
    assert!(null_strings.data().is_empty());
    assert_eq!(null_strings.offsets(), &[0, 0, 0, 0]);
    assert_eq!(
        null_strings.validity().iter().by_vals().collect::<Vec<_>>(),
        [false, false, false]
    );
}

#[test]
fn checkpoint_4_erases_arrays_through_the_two_row_catalog() {
    assert_eq!(
        PHYSICAL_FAMILY_CATALOG
            .iter()
            .filter(|entry| matches!(entry.family, PhysicalFamily::Int32 | PhysicalFamily::String))
            .map(|entry| (entry.family, entry.name))
            .collect::<Vec<_>>(),
        [
            (PhysicalFamily::Int32, "Int32"),
            (PhysicalFamily::String, "String"),
        ],
        "implement for_each_physical_family with the Int32 and String rows; Chapter 2 expands it"
    );

    let integers: ArrayImpl = I32Array::from_slice(&[Some(1), None]).into();
    assert_eq!(
        I32Array::try_from(integers)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(1), None]
    );

    let strings: ArrayImpl = StringArray::from_slice(&[Some("a"), Some("b")]).into();
    assert_eq!(
        <&StringArray>::try_from(&strings).unwrap().get(1),
        Some("b")
    );
    assert!(<&I32Array>::try_from(&strings).is_err());
}
