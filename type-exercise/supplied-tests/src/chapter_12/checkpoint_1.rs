use crate::*;

pub(super) fn i32_list(values: &[Option<i32>]) -> ListScalar {
    ListScalar::try_new(I32Array::from_slice(values).into()).unwrap()
}

#[test]
fn logical_list_keeps_its_element_physical_type() {
    let data_type = DataType::List(Box::new(DataType::Integer));
    assert_eq!(
        data_type.physical_type(),
        PhysicalType::List(Box::new(PhysicalType::Int32))
    );
}

#[test]
fn owned_and_borrowed_lists_slice_without_leaking_other_items() {
    let value = i32_list(&[Some(1), None, Some(3)]);
    let middle = value.as_list_ref().slice(1, 3).unwrap();
    assert_eq!(middle.len(), 2);
    assert_eq!(middle.get(0).unwrap(), None);
    assert_eq!(middle.get(1).unwrap(), Some(ScalarRefImpl::Int32(3)));
    assert_eq!(
        middle.to_owned_scalar().unwrap().get(1).unwrap(),
        Some(ScalarRefImpl::Int32(3))
    );
    assert!(value.as_list_ref().slice(2, 4).is_err());
}
