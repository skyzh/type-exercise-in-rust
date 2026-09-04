use crate::{
    Array, ArrayImpl, BoolArray, ColumnView, ColumnViewImpl, F64Array, I16Array, I32Array,
    PhysicalType, ScalarRefImpl, StringArray,
};

#[test]
fn reads_array_constant_null_and_indexed_rows_lazily() {
    let integers: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let array = ColumnView::<i32>::try_from(ColumnViewImpl::array(&integers)).unwrap();
    assert_eq!(
        (0..array.len())
            .map(|row| array.get(row))
            .collect::<Vec<_>>(),
        vec![Some(10), None, Some(30)]
    );

    let constant =
        ColumnView::<String>::try_from(ColumnViewImpl::constant(ScalarRefImpl::String("same"), 3))
            .unwrap();
    assert_eq!(
        (0..3).map(|row| constant.get(row)).collect::<Vec<_>>(),
        vec![Some("same"); 3]
    );

    let null = ColumnView::<f64>::try_from(ColumnViewImpl::null(PhysicalType::Float64, 2)).unwrap();
    assert_eq!(
        (0..2).map(|row| null.get(row)).collect::<Vec<_>>(),
        vec![None, None]
    );

    let words: ArrayImpl = StringArray::from_slice(&[Some("red"), None, Some("green")]).into();
    let indexed =
        ColumnView::<String>::try_from(ColumnViewImpl::indexed(&[2, 1, 0, 2], &words).unwrap())
            .unwrap();
    assert_eq!(
        (0..indexed.len())
            .map(|row| indexed.get(row))
            .collect::<Vec<_>>(),
        vec![Some("green"), None, Some("red"), Some("green")]
    );
}

#[test]
fn keeps_types_for_empty_and_all_null_views() {
    let empty = ColumnViewImpl::null(PhysicalType::Int32, 0);
    assert!(empty.is_empty());
    assert_eq!(empty.physical_type(), PhysicalType::Int32);

    let booleans: ArrayImpl = BoolArray::from_slice(&[Some(false)]).into();
    assert_eq!(
        ColumnViewImpl::array(&booleans).physical_type(),
        PhysicalType::Bool
    );
    let smallints: ArrayImpl = I16Array::from_slice(&[Some(-1), None]).into();
    assert_eq!(
        ColumnViewImpl::array(&smallints).physical_type(),
        PhysicalType::Int16
    );
    let doubles: ArrayImpl = F64Array::from_slice(&[Some(-0.0)]).into();
    assert_eq!(
        ColumnViewImpl::array(&doubles).physical_type(),
        PhysicalType::Float64
    );
}

#[test]
fn rejects_invalid_indices_and_family_mismatches_before_row_access() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    assert!(ColumnViewImpl::indexed(&[0, 1], &values).is_err());
    assert!(ColumnView::<String>::try_from(ColumnViewImpl::array(&values)).is_err());
    assert!(ColumnView::<String>::try_from(ColumnViewImpl::null(PhysicalType::Int32, 1)).is_err());
}
