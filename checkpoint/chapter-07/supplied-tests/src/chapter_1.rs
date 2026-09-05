use bitvec::vec::BitVec;

use crate::*;

#[test]
fn defines_the_physical_family_and_logical_type_catalogs() {
    let families = PHYSICAL_FAMILY_CATALOG
        .iter()
        .map(|entry| (entry.family, entry.name))
        .collect::<Vec<_>>();
    assert_eq!(
        families,
        [
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

    let decimal_type = DecimalType::try_new(8, 2).unwrap();
    assert_eq!(DataType::SmallInt.physical_type(), PhysicalType::Int16);
    assert_eq!(DataType::Integer.physical_type(), PhysicalType::Int32);
    assert_eq!(DataType::BigInt.physical_type(), PhysicalType::Int64);
    assert_eq!(DataType::Boolean.physical_type(), PhysicalType::Bool);
    assert_eq!(DataType::Real.physical_type(), PhysicalType::Float32);
    assert_eq!(DataType::Double.physical_type(), PhysicalType::Float64);
    assert_eq!(DataType::Varchar.physical_type(), PhysicalType::String);
    assert_eq!(
        DataType::Decimal(decimal_type).physical_type(),
        PhysicalType::Decimal(decimal_type)
    );
}

#[test]
fn builds_and_reads_every_fixed_width_array() {
    let i16s = I16Array::from_slice(&[Some(-2), None, Some(7)]);
    assert_eq!(i16s.values(), &[-2, 0, 7]);
    assert_eq!(i16s.iter().collect::<Vec<_>>(), [Some(-2), None, Some(7)]);

    let i32s = I32Array::from_slice(&[Some(10), None, Some(30)]);
    let validity: &BitVec = i32s.validity();
    assert_eq!(i32s.values(), &[10, 0, 30]);
    assert_eq!(
        validity.iter().by_vals().collect::<Vec<_>>(),
        [true, false, true]
    );

    assert_eq!(
        I64Array::from_slice(&[Some(i64::MAX)]).get(0),
        Some(i64::MAX)
    );
    assert_eq!(BoolArray::from_slice(&[Some(true), None]).get(1), None);
    assert_eq!(F32Array::from_slice(&[Some(1.25)]).get(0), Some(1.25));
    assert_eq!(F64Array::from_slice(&[Some(-9.5)]).get(0), Some(-9.5));
}

#[test]
fn stores_strings_as_bytes_offsets_and_validity() {
    let strings = StringArray::from_slice(&[Some("a"), None, Some("é"), Some("")]);
    assert_eq!(strings.data(), "aé".as_bytes());
    assert_eq!(strings.offsets(), &[0, 1, 1, 3, 3]);
    assert_eq!(
        strings.validity().iter().by_vals().collect::<Vec<_>>(),
        [true, false, true, true]
    );
    assert_eq!(
        strings.iter().collect::<Vec<_>>(),
        [Some("a"), None, Some("é"), Some("")]
    );

    let borrowed = strings.get(2).unwrap();
    assert_eq!(
        borrowed.as_ptr(),
        strings.data()[strings.offsets()[2]..].as_ptr()
    );
}

#[test]
fn keeps_one_checked_decimal_descriptor_per_array() {
    assert!(DecimalType::try_new(0, 0).is_err());
    assert!(DecimalType::try_new(3, 4).is_err());

    let money = DecimalType::try_new(6, 2).unwrap();
    let other = DecimalType::try_new(6, 3).unwrap();
    let first = Decimal::try_new(12_345, money).unwrap();
    let second = Decimal::try_new(-50, money).unwrap();
    let array = DecimalArray::try_from_slice(money, &[Some(first), None, Some(second)]).unwrap();
    assert_eq!(array.decimal_type(), money);
    assert_eq!(array.values(), &[12_345, 0, -50]);
    assert_eq!(array.get(0), Some(first));
    assert_eq!(array.get(1), None);

    let mut builder = DecimalArrayBuilder::try_with_type(money, 2).unwrap();
    builder.try_push(Some(first)).unwrap();
    assert!(
        builder
            .try_push(Some(Decimal::try_new(7, other).unwrap()))
            .is_err()
    );
    assert_eq!(
        builder.len(),
        1,
        "a rejected row must not mutate the builder"
    );
}

#[test]
fn erases_and_recovers_dense_values_with_checked_types() {
    let integer: ArrayImpl = I32Array::from_slice(&[Some(4), None]).into();
    assert_eq!(integer.physical_type(), PhysicalType::Int32);
    assert_eq!(integer.get(0), Some(ScalarRefImpl::Int32(4)));
    assert_eq!(integer.get(1), None);
    assert!(<&StringArray>::try_from(&integer).is_err());
    assert_eq!(<&I32Array>::try_from(&integer).unwrap().get(0), Some(4));

    let text = ScalarImpl::from(String::from("dense"));
    assert_eq!(text.physical_type(), PhysicalType::String);
    assert_eq!(String::try_from(text).unwrap(), "dense");
    assert!(i32::try_from(ScalarImpl::from(String::from("wrong"))).is_err());

    assert!(
        F32Array::from_slice(&[Some(f32::NAN)])
            .get(0)
            .unwrap()
            .is_nan()
    );
    assert_eq!(
        F64Array::from_slice(&[Some(f64::INFINITY)]).get(0),
        Some(f64::INFINITY)
    );
}
