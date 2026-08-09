use crate::{
    Array, ArrayBuilder, ArrayImpl, BoolArray, DataType, Decimal, DecimalArray, F32Array, F64Array,
    I16Array, I32Array, I64Array, PHYSICAL_FAMILY_CATALOG, PhysicalType, Scalar, ScalarImpl,
    ScalarRef, ScalarRefImpl, StringArray, StringArrayBuilder, TypeMismatch,
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
fn connects_owned_borrowed_and_array_types() {
    assert_complete_family::<i16, I16Array>();
    assert_complete_family::<i32, I32Array>();
    assert_complete_family::<i64, I64Array>();
    assert_complete_family::<bool, BoolArray>();
    assert_complete_family::<f32, F32Array>();
    assert_complete_family::<f64, F64Array>();
    assert_complete_family::<String, StringArray>();
    assert_complete_family::<Decimal, DecimalArray>();

    assert_eq!(DataType::Double.physical_type(), PhysicalType::Float64);

    let integer = 42_i32;
    let integer_ref: <i32 as Scalar>::RefType<'_> = integer.as_scalar_ref();
    assert_eq!(integer_ref, 42);
    assert_eq!(integer_ref.to_owned_scalar(), 42);

    let string = String::from("type system");
    let string_ref: <String as Scalar>::RefType<'_> = string.as_scalar_ref();
    assert_eq!(string_ref, "type system");
    assert_eq!(string_ref.to_owned_scalar(), string);

    let integers: <i32 as Scalar>::ArrayType = I32Array::from_slice(&[Some(1), None, Some(3)]);
    let strings: <String as Scalar>::ArrayType =
        StringArray::from_slice(&[Some("one"), None, Some("three")]);
    assert_eq!(integers.get(0), Some(1));
    assert_eq!(strings.get(0), Some("one"));

    let source = String::from("owned by the array");
    let mut builder = StringArrayBuilder::with_capacity(1);
    builder.push(Some(source.as_str()));
    drop(source);
    let built = builder.finish();
    assert_eq!(built.get(0), Some("owned by the array"));
}

#[test]
fn keeps_every_physical_family_in_the_single_checked_catalog() {
    assert_eq!(
        PHYSICAL_FAMILY_CATALOG
            .iter()
            .map(|family| (family.physical_type, family.name))
            .collect::<Vec<_>>(),
        vec![
            (PhysicalType::Int16, "Int16"),
            (PhysicalType::Int32, "Int32"),
            (PhysicalType::Int64, "Int64"),
            (PhysicalType::Bool, "Bool"),
            (PhysicalType::Float32, "Float32"),
            (PhysicalType::Float64, "Float64"),
            (PhysicalType::String, "String"),
            (PhysicalType::Decimal, "Decimal"),
        ]
    );
}

#[test]
fn maps_every_logical_type_without_enforcing_logical_parameters() {
    let mappings = [
        (DataType::SmallInt, PhysicalType::Int16),
        (DataType::Integer, PhysicalType::Int32),
        (DataType::BigInt, PhysicalType::Int64),
        (DataType::Boolean, PhysicalType::Bool),
        (DataType::Real, PhysicalType::Float32),
        (DataType::Double, PhysicalType::Float64),
        (DataType::Varchar, PhysicalType::String),
        (DataType::Char { width: 7 }, PhysicalType::String),
        (
            DataType::Decimal {
                scale: 2,
                precision: 8,
            },
            PhysicalType::Decimal,
        ),
    ];
    for (logical, physical) in mappings {
        assert_eq!(logical.physical_type(), physical);
    }

    assert_eq!(
        DataType::Char { width: 0 }.physical_type(),
        DataType::Char { width: u16::MAX }.physical_type()
    );
    assert_eq!(
        DataType::Decimal {
            scale: 0,
            precision: 0,
        }
        .physical_type(),
        DataType::Decimal {
            scale: u16::MAX,
            precision: u16::MAX,
        }
        .physical_type()
    );
}

#[test]
fn preserves_special_float_values_across_the_explicit_double_family() {
    let values = F64Array::from_slice(&[Some(f64::NAN), Some(f64::INFINITY), Some(-0.0), None]);

    assert!(values.get(0).unwrap().is_nan());
    assert_eq!(values.get(1), Some(f64::INFINITY));
    assert_eq!(values.get(2).unwrap().to_bits(), (-0.0_f64).to_bits());
    assert_eq!(values.get(3), None);

    let erased_scalar = ScalarImpl::from(-0.0_f64);
    assert_eq!(
        f64::try_from(erased_scalar).unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
    let erased_array = ArrayImpl::from(values.clone());
    let restored = F64Array::try_from(erased_array).unwrap();
    assert!(restored.get(0).unwrap().is_nan());
    assert_eq!(restored.get(2).unwrap().to_bits(), (-0.0_f64).to_bits());

    let singles = F32Array::from_slice(&[Some(f32::NAN), Some(f32::INFINITY), Some(-0.0)]);
    let restored = F32Array::try_from(ArrayImpl::from(singles)).unwrap();
    assert!(restored.get(0).unwrap().is_nan());
    assert_eq!(restored.get(1), Some(f32::INFINITY));
    assert_eq!(restored.get(2).unwrap().to_bits(), (-0.0_f32).to_bits());
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

    let empty = StringArray::from_slice(&[]);
    assert!(empty.is_empty());
    assert_eq!(empty.iter().next(), None);
}

#[test]
fn round_trips_erased_values_and_arrays() {
    assert_eq!(i16::try_from(ScalarImpl::from(-7_i16)).unwrap(), -7);
    let integer = i32::try_from(ScalarImpl::from(9_i32)).unwrap();
    assert_eq!(integer, 9);
    assert_eq!(i64::try_from(ScalarImpl::from(11_i64)).unwrap(), 11);
    assert!(bool::try_from(ScalarImpl::from(true)).unwrap());
    assert_eq!(f32::try_from(ScalarImpl::from(1.5_f32)).unwrap(), 1.5);
    assert_eq!(f64::try_from(ScalarImpl::from(2.5_f64)).unwrap(), 2.5);

    let decimal = Decimal::new(1234, 2);
    assert_eq!(
        Decimal::try_from(ScalarImpl::from(decimal)).unwrap(),
        decimal
    );

    let string = String::try_from(ScalarImpl::from(String::from("owned"))).unwrap();
    assert_eq!(string, "owned");

    let borrowed = i32::try_from(ScalarRefImpl::from(7_i32)).unwrap();
    assert_eq!(borrowed, 7);

    let borrowed = <&str>::try_from(ScalarRefImpl::from("borrowed")).unwrap();
    assert_eq!(borrowed, "borrowed");

    let erased: ArrayImpl = I32Array::from_slice(&[Some(1), None]).into();
    let typed = I32Array::try_from(erased).unwrap();
    assert_eq!(typed.iter().collect::<Vec<_>>(), vec![Some(1), None]);

    let erased: ArrayImpl = StringArray::from_slice(&[Some("a"), Some("b")]).into();
    let typed = <&StringArray>::try_from(&erased).unwrap();
    assert_eq!(typed.get(1), Some("b"));

    let erased: ArrayImpl = BoolArray::from_slice(&[Some(true), None]).into();
    assert_eq!(
        BoolArray::try_from(erased)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(true), None]
    );

    let erased: ArrayImpl = DecimalArray::from_slice(&[Some(decimal), None]).into();
    assert_eq!(
        DecimalArray::try_from(erased)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(decimal), None]
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
