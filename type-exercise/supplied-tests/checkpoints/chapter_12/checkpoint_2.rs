use crate::{
    Array, ArrayImpl, DataType, I32Array, ListArray, ListScalar, PhysicalType, ScalarRefImpl,
    StringArray,
};

fn i32_list(values: &[Option<i32>]) -> ListScalar {
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
fn empty_null_and_nonempty_rows_have_exact_outer_invariants() {
    let empty = i32_list(&[]);
    let values = i32_list(&[Some(7), None, Some(9)]);
    let array = ListArray::try_from_rows(
        PhysicalType::Int32,
        [Some(empty.as_list_ref()), None, Some(values.as_list_ref())],
    )
    .unwrap();

    assert_eq!(array.len(), 3);
    assert_eq!(array.offsets(), &[0, 0, 0, 3]);
    assert_eq!(array.validity(), &[true, false, true]);
    assert_eq!(array.values().len(), 3);
    assert_eq!(array.get(0).unwrap().unwrap().len(), 0);
    assert_eq!(array.get(1).unwrap(), None);
    let last = array.get(2).unwrap().unwrap();
    assert_eq!(last.get(0).unwrap(), Some(ScalarRefImpl::Int32(7)));
    assert_eq!(last.get(1).unwrap(), None);
    assert_eq!(last.get(2).unwrap(), Some(ScalarRefImpl::Int32(9)));

    let sliced = array.slice(1, 3).unwrap();
    assert_eq!(sliced.offsets(), &[0, 0, 3]);
    assert_eq!(sliced.validity(), &[false, true]);
    assert_eq!(sliced.values().len(), 3);
}

#[test]
fn empty_and_all_null_arrays_retain_their_child_type() {
    let empty = ListArray::try_from_rows(PhysicalType::String, []).unwrap();
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.offsets(), &[0]);
    assert_eq!(empty.values().physical_type(), PhysicalType::String);

    let all_null = ListArray::try_from_rows(PhysicalType::Float64, [None, None]).unwrap();
    assert_eq!(all_null.len(), 2);
    assert_eq!(all_null.offsets(), &[0, 0, 0]);
    assert_eq!(all_null.validity(), &[false, false]);
    assert_eq!(all_null.values().physical_type(), PhysicalType::Float64);
}

#[test]
fn raw_parts_reject_bad_types_offsets_and_null_spans() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2)]).into();
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::String,
            values.clone(),
            vec![0, 2],
            vec![true],
        )
        .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(PhysicalType::Int32, values.clone(), vec![1, 2], vec![true],)
            .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(PhysicalType::Int32, values.clone(), vec![0], vec![true],)
            .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::Int32,
            values.clone(),
            vec![0, 2],
            vec![true, true],
        )
        .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::Int32,
            values.clone(),
            vec![0, 2, 1],
            vec![true, true],
        )
        .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::Int32,
            values.clone(),
            vec![0, 2, 1, 2],
            vec![true, true, true],
        )
        .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::Int32,
            values.clone(),
            vec![0, 2],
            vec![false],
        )
        .is_err()
    );
    assert!(
        ListArray::try_from_raw_parts(PhysicalType::Int32, values, vec![0, 3], vec![true],)
            .is_err()
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

#[test]
fn nested_list_construction_is_rejected_explicitly() {
    let child = ListArray::try_from_rows(PhysicalType::Int32, [None]).unwrap();
    assert!(ListScalar::try_new(child.clone().into()).is_err());
    assert!(
        ListArray::try_from_raw_parts(
            PhysicalType::List(Box::new(PhysicalType::Int32)),
            child.into(),
            vec![0],
            vec![],
        )
        .is_err()
    );
}

#[test]
fn row_type_error_does_not_return_a_partial_array() {
    let integers = i32_list(&[Some(1)]);
    let strings = ListScalar::try_new(StringArray::from_slice(&[Some("wrong")]).into()).unwrap();
    assert!(
        ListArray::try_from_rows(
            PhysicalType::Int32,
            [Some(integers.as_list_ref()), Some(strings.as_list_ref())],
        )
        .is_err()
    );
}
