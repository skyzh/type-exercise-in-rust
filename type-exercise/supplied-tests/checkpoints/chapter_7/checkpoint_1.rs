use crate::{Array, ArrayImpl, ColumnViewImpl, I32Array, PhysicalType, ScalarRefImpl};

#[test]
fn checkpoint_1_preserves_array_constant_and_null_rows() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(4), None, Some(8)]).into();
    let array = ColumnViewImpl::array(&values);
    assert_eq!(array.len(), 3);
    assert_eq!(array.physical_type(), PhysicalType::Int32);
    assert_eq!(array.get(0), Some(ScalarRefImpl::Int32(4)));
    assert_eq!(array.get(1), None);

    let constant = ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 3);
    assert_eq!(constant.len(), 3);
    assert_eq!(constant.get(2), Some(ScalarRefImpl::Int32(7)));

    let null = ColumnViewImpl::null(PhysicalType::Int32, 3);
    assert_eq!(null.len(), 3);
    assert_eq!(null.get(2), None);
}

#[test]
fn checkpoint_1_preserves_indexed_order_nulls_and_bounds() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(4), None, Some(8)]).into();
    let keys = [2, 1, 0];
    let indexed = ColumnViewImpl::indexed(&keys, &values).unwrap();
    assert_eq!(indexed.len(), 3);
    assert_eq!(indexed.physical_type(), PhysicalType::Int32);
    assert_eq!(indexed.get(0), Some(ScalarRefImpl::Int32(8)));
    assert_eq!(indexed.get(1), None);
    assert_eq!(indexed.get(2), Some(ScalarRefImpl::Int32(4)));

    assert_eq!(
        ColumnViewImpl::indexed(&[3], &values)
            .unwrap_err()
            .to_string(),
        "index 3 at row 0 is out of bounds for a values array of length 3"
    );
}
