use crate::{
    Array, ArrayBuilder, ArrayImpl, I32Array, PhysicalType, Scalar, ScalarImpl, ScalarRef,
    ScalarRefImpl, StringArray, StringArrayBuilder, TypeMismatch,
};

fn assert_complete_family<S, A>()
where
    S: Scalar<ArrayType = A>,
    A: Array<OwnedItem = S>,
    for<'a> S: Scalar<RefType<'a> = A::RefItem<'a>>,
    for<'a> A::RefItem<'a>: ScalarRef<'a, ScalarType = S, ArrayType = A>,
    A::Builder: ArrayBuilder<Array = A>,
{
}

#[test]
fn connects_the_explicit_integer_and_string_families() {
    assert_complete_family::<i32, I32Array>();
    assert_complete_family::<String, StringArray>();

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
fn builds_nullable_integer_and_string_arrays() {
    let integers = I32Array::from_slice(&[Some(10), None, Some(30)]);
    assert_eq!(integers.len(), 3);
    assert_eq!(
        integers.iter().collect::<Vec<_>>(),
        vec![Some(10), None, Some(30)]
    );

    let strings = StringArray::from_slice(&[Some("db"), None, Some("rust")]);
    assert_eq!(strings.len(), 3);
    assert_eq!(
        strings.iter().collect::<Vec<_>>(),
        vec![Some("db"), None, Some("rust")]
    );

    let source = String::from("owned by the array");
    let mut builder = StringArrayBuilder::with_capacity(1);
    builder.push(Some(source.as_str()));
    drop(source);
    assert_eq!(builder.finish().get(0), Some("owned by the array"));
}

#[test]
fn round_trips_the_two_explicit_erased_families() {
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
}

#[test]
fn rejects_mismatched_erased_values_and_arrays() {
    assert_eq!(
        i32::try_from(ScalarImpl::String("wrong".to_owned())),
        Err(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        })
    );
    assert_eq!(
        <&str>::try_from(ScalarRefImpl::Int32(1)),
        Err(TypeMismatch {
            expected: PhysicalType::String,
            actual: PhysicalType::Int32,
        })
    );

    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    assert_eq!(
        <&I32Array>::try_from(&strings).unwrap_err(),
        TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }
    );
}
