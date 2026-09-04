use bitvec::prelude::{Lsb0, bitvec};

use crate::{
    Array, ArrayBuilder, ArrayImpl, BoolArray, DataType, Decimal, DecimalArray,
    DecimalArrayBuilder, DecimalType, F32Array, F64Array, I16Array, I32Array, I64Array,
    PHYSICAL_FAMILY_CATALOG, PhysicalFamily, PhysicalType, Scalar, ScalarImpl, ScalarRef,
    ScalarRefImpl, StringArray,
};

fn assert_family<S, A>()
where
    S: Scalar<ArrayType = A>,
    A: Array<OwnedItem = S>,
    for<'a> S: Scalar<RefType<'a> = A::RefItem<'a>>,
    for<'a> A::RefItem<'a>: ScalarRef<'a, ScalarType = S, ArrayType = A>,
    A::Builder: ArrayBuilder<Array = A>,
{
}

#[test]
fn connects_every_static_scalar_and_array_family() {
    assert_family::<i16, I16Array>();
    assert_family::<i32, I32Array>();
    assert_family::<i64, I64Array>();
    assert_family::<bool, BoolArray>();
    assert_family::<f32, F32Array>();
    assert_family::<f64, F64Array>();
    assert_family::<String, StringArray>();

    assert_eq!(
        PHYSICAL_FAMILY_CATALOG
            .iter()
            .map(|entry| (entry.family, entry.name))
            .collect::<Vec<_>>(),
        vec![
            (PhysicalFamily::Int16, "Int16"),
            (PhysicalFamily::Int32, "Int32"),
            (PhysicalFamily::Int64, "Int64"),
            (PhysicalFamily::Bool, "Bool"),
            (PhysicalFamily::Float32, "Float32"),
            (PhysicalFamily::Float64, "Float64"),
            (PhysicalFamily::String, "String"),
            (PhysicalFamily::Decimal, "Decimal"),
        ]
    );
}

#[test]
fn stores_nullable_fixed_and_variable_width_arrays() {
    let integers = I32Array::from_slice(&[Some(7), None, Some(-3)]);
    assert_eq!(integers.values(), &[7, 0, -3]);
    assert_eq!(integers.validity(), &bitvec![1, 0, 1]);
    assert_eq!(
        integers.iter().collect::<Vec<_>>(),
        vec![Some(7), None, Some(-3)]
    );

    let strings = StringArray::from_slice(&[Some("db"), None, Some("数据")]);
    assert_eq!(strings.data(), b"db\xE6\x95\xB0\xE6\x8D\xAE");
    assert_eq!(strings.offsets(), &[0, 2, 2, 8]);
    assert_eq!(strings.validity(), &bitvec![1, 0, 1]);
    assert_eq!(
        strings.iter().collect::<Vec<_>>(),
        vec![Some("db"), None, Some("数据")]
    );
}

#[test]
fn erases_and_recovers_arrays_and_scalars_with_checked_types() {
    let array = I16Array::from_slice(&[Some(4), None]);
    let erased = ArrayImpl::from(array.clone());
    assert_eq!(erased.physical_type(), PhysicalType::Int16);
    assert_eq!(<&I16Array>::try_from(&erased).unwrap(), &array);
    assert!(StringArray::try_from(erased).is_err());

    let scalar = ScalarImpl::from(String::from("rust"));
    assert_eq!(scalar.physical_type(), PhysicalType::String);
    assert_eq!(String::try_from(scalar).unwrap(), "rust");
    assert!(i32::try_from(ScalarRefImpl::String("wrong family")).is_err());
}

#[test]
fn keeps_decimal_metadata_with_values_and_logical_types() {
    let decimal_type = DecimalType::try_new(6, 2).unwrap();
    let value = Decimal::try_new(12_345, decimal_type).unwrap();
    let array = DecimalArray::try_from_slice(decimal_type, &[Some(value), None]).unwrap();
    assert_eq!(array.values(), &[12_345, 0]);
    assert_eq!(array.validity(), &bitvec![1, 0]);
    assert_eq!(array.decimal_type(), decimal_type);
    assert_eq!(array.get(0), Some(value));

    let other_type = DecimalType::try_new(7, 2).unwrap();
    let mut builder = DecimalArrayBuilder::try_with_type(decimal_type, 2).unwrap();
    builder.try_push(Some(value)).unwrap();
    let before = builder.clone();
    assert!(
        builder
            .try_push(Some(Decimal::try_new(12_345, other_type).unwrap()))
            .is_err()
    );
    assert_eq!(builder, before);

    assert_eq!(DataType::Integer.physical_type(), PhysicalType::Int32);
    assert_eq!(DataType::Varchar.physical_type(), PhysicalType::String);
    assert_eq!(
        DataType::Decimal(decimal_type).physical_type(),
        PhysicalType::Decimal(decimal_type)
    );
}

#[test]
fn preserves_special_float_values() {
    let values = F64Array::from_slice(&[Some(f64::NAN), Some(f64::INFINITY), Some(-0.0), None]);
    let restored = F64Array::try_from(ArrayImpl::from(values)).unwrap();
    assert!(restored.get(0).unwrap().is_nan());
    assert_eq!(restored.get(1), Some(f64::INFINITY));
    assert_eq!(restored.get(2).unwrap().to_bits(), (-0.0_f64).to_bits());
    assert_eq!(restored.get(3), None);

    let singles = F32Array::from_slice(&[Some(f32::NAN), Some(-0.0)]);
    assert!(singles.get(0).unwrap().is_nan());
    assert_eq!(singles.get(1).unwrap().to_bits(), (-0.0_f32).to_bits());
}
