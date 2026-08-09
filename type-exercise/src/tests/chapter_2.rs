use crate::{
    Array, ArrayBuilder, ArrayImpl, BoolArray, DataType, Decimal, DecimalArray, F32Array, F64Array,
    I16Array, I32Array, I64Array, PHYSICAL_FAMILY_CATALOG, PhysicalType, Scalar, ScalarImpl,
    ScalarRef, StringArray,
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
fn one_catalog_connects_every_primitive_family() {
    assert_complete_family::<i16, I16Array>();
    assert_complete_family::<i32, I32Array>();
    assert_complete_family::<i64, I64Array>();
    assert_complete_family::<bool, BoolArray>();
    assert_complete_family::<f32, F32Array>();
    assert_complete_family::<f64, F64Array>();
    assert_complete_family::<String, StringArray>();
    assert_complete_family::<Decimal, DecimalArray>();

    assert_eq!(
        PHYSICAL_FAMILY_CATALOG
            .iter()
            .map(|family| (family.physical_type.clone(), family.name))
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
fn maps_every_logical_type_without_claiming_parameter_validation() {
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
            DataType::Decimal {
                scale: 2,
                precision: 8,
            },
            PhysicalType::Decimal,
        ),
    ] {
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

    let decimal = Decimal::new(1234, 2);
    assert_eq!(
        Decimal::try_from(ScalarImpl::from(decimal)).unwrap(),
        decimal
    );
    assert_eq!(
        BoolArray::try_from(ArrayImpl::from(BoolArray::from_slice(&[Some(true), None])))
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(true), None]
    );
}
