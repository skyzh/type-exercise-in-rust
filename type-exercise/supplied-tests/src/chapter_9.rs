use crate::{
    Array, ArrayImpl, ColumnViewImpl, DataType, I32Array, ListArray, ListScalar, PhysicalType,
    ScalarRefImpl,
};

fn i32_list(values: &[Option<i32>]) -> ListScalar {
    ListScalar::try_new(I32Array::from_slice(values).into()).unwrap()
}

#[test]
fn stores_one_level_lists_with_distinct_null_and_empty_rows() {
    let empty = i32_list(&[]);
    let values = i32_list(&[Some(7), None, Some(9)]);
    let array = ListArray::try_from_rows(
        PhysicalType::Int32,
        [Some(empty.as_list_ref()), None, Some(values.as_list_ref())],
    )
    .unwrap();
    assert_eq!(array.offsets(), &[0, 0, 0, 3]);
    assert_eq!(array.validity(), &[true, false, true]);
    assert_eq!(array.get(0).unwrap().unwrap().len(), 0);
    assert_eq!(array.get(1).unwrap(), None);
    assert_eq!(array.get(2).unwrap().unwrap().get(1).unwrap(), None);
    assert_eq!(
        DataType::List(Box::new(DataType::Integer)).physical_type(),
        PhysicalType::List(Box::new(PhysicalType::Int32))
    );
}

#[test]
fn slices_and_recovers_array_constant_null_and_indexed_list_views() {
    let first = i32_list(&[Some(10), None]);
    let second = i32_list(&[]);
    let lists: ArrayImpl = ListArray::try_from_rows(
        PhysicalType::Int32,
        [Some(first.as_list_ref()), Some(second.as_list_ref()), None],
    )
    .unwrap()
    .into();
    let array = ColumnViewImpl::array(&lists)
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(array.get(0).unwrap().unwrap().len(), 2);
    assert_eq!(array.get(2).unwrap(), None);

    let constant = ColumnViewImpl::constant(ScalarRefImpl::List(first.as_list_ref()), 2)
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(constant.get(1).unwrap().unwrap().len(), 2);

    let indices = [1, 2, 0];
    let indexed = ColumnViewImpl::indexed(&indices, &lists)
        .unwrap()
        .try_as_list(PhysicalType::Int32)
        .unwrap();
    assert_eq!(indexed.get(0).unwrap().unwrap().len(), 0);
    assert_eq!(indexed.get(1).unwrap(), None);
}

#[test]
fn rejects_wrong_child_types_nested_lists_and_invalid_raw_parts() {
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
        ListArray::try_from_raw_parts(PhysicalType::Int32, values, vec![0, 2], vec![false],)
            .is_err()
    );
    let child = ListArray::try_from_rows(PhysicalType::Int32, [None]).unwrap();
    assert!(ListScalar::try_new(child.into()).is_err());
}
