use crate::*;

#[test]
fn array_views_preserve_type_length_values_and_nulls() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let view = ColumnViewImpl::array(&values);

    assert_eq!(view.physical_type(), PhysicalType::Int32);
    assert_eq!(view.len(), 3);
    assert!(!view.is_empty());
    assert_eq!(view.get(0), Some(ScalarRefImpl::Int32(10)));
    assert_eq!(view.get(1), None);
    assert_eq!(view.get(2), Some(ScalarRefImpl::Int32(30)));

    let empty_values: ArrayImpl = I32Array::from_slice(&[]).into();
    assert!(ColumnViewImpl::array(&empty_values).is_empty());
}

#[test]
fn constant_views_repeat_values_and_typed_nulls() {
    let repeated = ColumnViewImpl::constant(ScalarRefImpl::String("db"), 3);
    assert_eq!(repeated.physical_type(), PhysicalType::String);
    assert_eq!(repeated.len(), 3);
    assert_eq!(repeated.get(0), Some(ScalarRefImpl::String("db")));
    assert_eq!(repeated.get(2), Some(ScalarRefImpl::String("db")));

    let nulls = ColumnViewImpl::null(PhysicalType::Int64, 2);
    assert_eq!(nulls.physical_type(), PhysicalType::Int64);
    assert_eq!(nulls.len(), 2);
    assert_eq!(nulls.get(0), None);
    assert_eq!(nulls.get(1), None);

    let typed = ColumnView::<i64>::try_from(nulls).unwrap();
    assert_eq!(typed.len(), 2);
    assert_eq!(typed.get(0), None);
}

#[test]
fn indexed_views_remap_rows_without_materializing_an_array() {
    let values: ArrayImpl = StringArray::from_slice(&[Some("zero"), None, Some("two")]).into();
    let indices = [2, 1, 2, 0];
    let view = ColumnViewImpl::indexed(&indices, &values).unwrap();

    assert_eq!(view.physical_type(), PhysicalType::String);
    assert_eq!(view.len(), indices.len());
    assert_eq!(view.get(0), Some(ScalarRefImpl::String("two")));
    assert_eq!(view.get(1), None);
    assert_eq!(view.get(2), Some(ScalarRefImpl::String("two")));
    assert_eq!(view.get(3), Some(ScalarRefImpl::String("zero")));

    let invalid = ColumnViewImpl::indexed(&[0, 3], &values).unwrap_err();
    assert!(invalid.to_string().contains("index 3 at row 1"));
}

#[test]
fn typed_views_check_the_family_once_and_borrow_the_same_storage() {
    let values: ArrayImpl = StringArray::from_slice(&[Some("rust"), None, Some("types")]).into();
    let erased = ColumnViewImpl::array(&values);
    assert!(ColumnView::<i32>::try_from(erased.clone()).is_err());

    let typed = ColumnView::<String>::try_from(erased).unwrap();
    assert_eq!(typed.len(), 3);
    assert_eq!(typed.get(0), Some("rust"));
    assert_eq!(typed.get(1), None);
    assert_eq!(typed.get(2), Some("types"));

    let decimal_type = DecimalType::try_new(6, 2).unwrap();
    let decimal = Decimal::try_new(12_345, decimal_type).unwrap();
    let decimals: ArrayImpl = DecimalArray::try_from_slice(decimal_type, &[Some(decimal), None])
        .unwrap()
        .into();
    let decimal_view = ColumnViewImpl::array(&decimals);
    assert_eq!(
        decimal_view.physical_type(),
        PhysicalType::Decimal(decimal_type)
    );
    assert_eq!(decimal_view.get(0), Some(ScalarRefImpl::Decimal(decimal)));
    assert_eq!(decimal_view.get(1), None);
}
