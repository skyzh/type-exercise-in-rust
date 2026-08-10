use bitvec::prelude::{Lsb0, bitvec};

use crate::{
    Array, ArrayBuilder, ArrayImpl, BoolArray, DataType, Decimal, DecimalArray,
    DecimalArrayBuilder, DecimalType, F32Array, F64Array, I16Array, I32Array, I64Array,
    PHYSICAL_FAMILY_CATALOG, PhysicalFamily, PhysicalType, Scalar, ScalarImpl, ScalarRef,
    StringArray,
};

fn assert_complete_static_family<S, A>()
where
    S: Scalar<ArrayType = A>,
    A: Array<OwnedItem = S>,
    for<'a> S: Scalar<RefType<'a> = A::RefItem<'a>>,
    for<'a> A::RefItem<'a>: ScalarRef<'a, ScalarType = S, ArrayType = A>,
    A::Builder: ArrayBuilder<Array = A>,
{
}

#[test]
fn one_catalog_connects_static_families_and_marks_decimal_dedicated() {
    assert_complete_static_family::<i16, I16Array>();
    assert_complete_static_family::<i32, I32Array>();
    assert_complete_static_family::<i64, I64Array>();
    assert_complete_static_family::<bool, BoolArray>();
    assert_complete_static_family::<f32, F32Array>();
    assert_complete_static_family::<f64, F64Array>();
    assert_complete_static_family::<String, StringArray>();

    assert_eq!(
        PHYSICAL_FAMILY_CATALOG
            .iter()
            .map(|family| (family.family, family.name))
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
fn validates_decimal_metadata_and_maps_exact_logical_types() {
    for (precision, scale) in [(1, 0), (38, 0), (38, 38)] {
        assert_eq!(
            DecimalType::try_new(precision, scale).unwrap(),
            DecimalType::try_new(precision, scale).unwrap()
        );
    }
    assert!(DecimalType::try_new(0, 0).is_err());
    assert!(DecimalType::try_new(39, 0).is_err());
    assert!(DecimalType::try_new(8, 9).is_err());

    let decimal_type = DecimalType::try_new(8, 2).unwrap();
    for (logical, physical) in [
        (DataType::SmallInt, PhysicalType::Int16),
        (DataType::Integer, PhysicalType::Int32),
        (DataType::BigInt, PhysicalType::Int64),
        (DataType::Boolean, PhysicalType::Bool),
        (DataType::Real, PhysicalType::Float32),
        (DataType::Double, PhysicalType::Float64),
        (DataType::Varchar, PhysicalType::String),
        (DataType::Char { width: 7 }, PhysicalType::String),
        (
            DataType::Decimal(decimal_type),
            PhysicalType::Decimal(decimal_type),
        ),
    ] {
        assert_eq!(logical.physical_type(), physical);
    }
    assert_eq!(DataType::decimal(8, 2), Ok(DataType::Decimal(decimal_type)));
    assert!(DataType::decimal(0, 0).is_err());
}

#[test]
fn decimal_array_uses_flat_coefficients_packed_validity_and_shared_metadata() {
    let decimal_type = DecimalType::try_new(6, 2).unwrap();
    let values = [
        Some(Decimal::try_new(12_345, decimal_type).unwrap()),
        None,
        Some(Decimal::try_new(-99, decimal_type).unwrap()),
    ];
    let array = DecimalArray::try_from_slice(decimal_type, &values).unwrap();

    let _: &[i128] = array.values();
    assert_eq!(array.values(), &[12_345, 0, -99]);
    assert_eq!(array.validity(), &bitvec![1, 0, 1]);
    assert_eq!(array.decimal_type(), decimal_type);
    assert_eq!(array.null_count(), 1);
    assert_eq!(array.get(0), values[0]);
    assert_eq!(array.get(1), None);
    let many = DecimalArray::try_from_slice(
        decimal_type,
        &vec![Some(Decimal::try_new(1, decimal_type).unwrap()); 130],
    )
    .unwrap();
    assert!(std::mem::size_of_val(many.validity().as_raw_slice()) < many.len());

    let empty = DecimalArray::try_from_slice(decimal_type, &[]).unwrap();
    let all_null = DecimalArray::try_from_slice(decimal_type, &[None, None]).unwrap();
    assert_eq!(empty.decimal_type(), decimal_type);
    assert_eq!(all_null.decimal_type(), decimal_type);
    assert_eq!(all_null.values(), &[0, 0]);
    assert_eq!(all_null.validity(), &bitvec![0, 0]);
}

#[test]
fn decimal_boundaries_and_builder_errors_fail_before_mutation() {
    let decimal_type = DecimalType::try_new(3, 1).unwrap();
    assert!(Decimal::try_new(999, decimal_type).is_ok());
    assert!(Decimal::try_new(-999, decimal_type).is_ok());
    assert!(Decimal::try_new(1_000, decimal_type).is_err());
    assert!(Decimal::try_new(-1_000, decimal_type).is_err());
    assert!(Decimal::try_new(i128::MIN, decimal_type).is_err());

    assert!(DecimalArray::try_from_raw_parts(decimal_type, vec![1, 2], bitvec![1]).is_err());
    assert!(DecimalArray::try_from_raw_parts(decimal_type, vec![1_000], bitvec![1]).is_err());

    let other_type = DecimalType::try_new(4, 1).unwrap();
    let mut builder = DecimalArrayBuilder::try_with_type(decimal_type, 2).unwrap();
    builder
        .try_push(Some(Decimal::try_new(10, decimal_type).unwrap()))
        .unwrap();
    let before = builder.clone();
    assert!(
        builder
            .try_push(Some(Decimal::try_new(10, other_type).unwrap()))
            .is_err()
    );
    assert_eq!(builder, before);
}

#[test]
fn decimal_erasure_preserves_exact_precision_and_scale() {
    let decimal_type = DecimalType::try_new(6, 2).unwrap();
    let other_type = DecimalType::try_new(6, 3).unwrap();
    let decimal = Decimal::try_new(1_234, decimal_type).unwrap();
    let erased = ScalarImpl::from(decimal);
    assert_eq!(erased.try_decimal(decimal_type), Ok(decimal));
    assert!(erased.try_decimal(other_type).is_err());
    let erased_ref = crate::ScalarRefImpl::from(decimal);
    assert_eq!(erased_ref.try_decimal(decimal_type), Ok(decimal));
    assert!(erased_ref.try_decimal(other_type).is_err());

    let array = DecimalArray::try_from_slice(decimal_type, &[Some(decimal), None]).unwrap();
    let erased = ArrayImpl::from(array.clone());
    assert_eq!(erased.physical_type(), PhysicalType::Decimal(decimal_type));
    assert_eq!(erased.try_decimal(decimal_type), Ok(&array));
    assert!(erased.try_decimal(other_type).is_err());
}

#[test]
fn preserves_special_float_values_and_checked_erasure() {
    let values = F64Array::from_slice(&[Some(f64::NAN), Some(f64::INFINITY), Some(-0.0), None]);
    let restored = F64Array::try_from(ArrayImpl::from(values)).unwrap();
    assert!(restored.get(0).unwrap().is_nan());
    assert_eq!(restored.get(1), Some(f64::INFINITY));
    assert_eq!(restored.get(2).unwrap().to_bits(), (-0.0_f64).to_bits());
    assert_eq!(restored.get(3), None);

    let singles = F32Array::from_slice(&[Some(f32::NAN), Some(f32::INFINITY), Some(-0.0)]);
    let restored = F32Array::try_from(ArrayImpl::from(singles)).unwrap();
    assert!(restored.get(0).unwrap().is_nan());
    assert_eq!(restored.get(2).unwrap().to_bits(), (-0.0_f32).to_bits());
}
