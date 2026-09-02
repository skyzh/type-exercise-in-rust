use super::checkpoint_1::i32_list;
use crate::*;

#[test]
fn erased_list_columns_cover_array_constant_null_and_indexed() {
    let first = i32_list(&[Some(10), None]);
    let second = i32_list(&[]);
    let array: ArrayImpl = ListArray::try_from_rows(
        PhysicalType::Int32,
        [Some(first.as_list_ref()), Some(second.as_list_ref()), None],
    )
    .unwrap()
    .into();
    assert_eq!(
        array.physical_type(),
        PhysicalType::List(Box::new(PhysicalType::Int32))
    );
    assert_eq!(<&ListArray>::try_from(&array).unwrap().len(), 3);

    let erased_scalar = ScalarImpl::List(first.clone());
    assert_eq!(
        <&ListScalar>::try_from(&erased_scalar)
            .unwrap()
            .element_type(),
        PhysicalType::Int32
    );

    let array_view = ColumnViewImpl::array(&array)
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(array_view.len(), 3);
    assert_eq!(array_view.get(0).unwrap().unwrap().len(), 2);
    assert_eq!(array_view.get(2).unwrap(), None);
    assert!(array_view.get(3).is_err());

    let constant = ColumnViewImpl::constant(ScalarRefImpl::List(first.as_list_ref()), 2)
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(constant.get(1).unwrap().unwrap().len(), 2);

    let null = ColumnViewImpl::null(PhysicalType::List(Box::new(PhysicalType::Int32)), 2)
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(null.get(0).unwrap(), None);

    let keys = [1, 2, 0];
    let indexed_view = ColumnViewImpl::indexed(&keys, &array)
        .unwrap()
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(indexed_view.get(0).unwrap().unwrap().len(), 0);
    assert_eq!(indexed_view.get(1).unwrap(), None);
    assert_eq!(indexed_view.get(2).unwrap().unwrap().len(), 2);

    assert!(
        ColumnViewImpl::array(&array)
            .try_as_list(PhysicalType::String)
            .is_err()
    );
}

#[test]
fn checked_list_column_rejects_nested_typed_nulls() {
    let nested_element = PhysicalType::List(Box::new(PhysicalType::Int32));
    let nested = ColumnViewImpl::null(PhysicalType::List(Box::new(nested_element.clone())), 2);
    assert_eq!(nested.len(), 2);
    assert!(nested.try_as_list(nested_element).is_err());

    let wrong_child = ColumnViewImpl::null(PhysicalType::List(Box::new(PhysicalType::String)), 2);
    assert!(wrong_child.try_as_list(PhysicalType::Int32).is_err());

    let one_level = ColumnViewImpl::null(PhysicalType::List(Box::new(PhysicalType::Int32)), 2)
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(one_level.get(0).unwrap(), None);
    assert_eq!(one_level.get(1).unwrap(), None);
}
