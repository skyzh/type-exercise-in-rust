use crate::{
    Array, ArrayImpl, BoolArray, ColumnView, ColumnViewImpl, F64Array, I16Array, I32Array,
    PhysicalType, ScalarRefImpl, StringArray,
};

#[test]
fn reads_arrays_constants_and_indexed_views_as_logical_rows() {
    let array: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let array_view = ColumnView::<i32>::try_from(ColumnViewImpl::array(&array)).unwrap();
    assert_eq!(array_view.len(), 3);
    assert_eq!(array_view.get(0), Some(10));
    assert_eq!(array_view.get(1), None);

    let erased_constant = ColumnViewImpl::constant(ScalarRefImpl::String("a"), 3);
    assert_eq!(erased_constant.physical_type(), PhysicalType::String);
    let constant = ColumnView::<String>::try_from(erased_constant).unwrap();
    assert_eq!(
        (0..constant.len())
            .map(|row| constant.get(row))
            .collect::<Vec<_>>(),
        vec![Some("a"), Some("a"), Some("a")]
    );

    let values: ArrayImpl = StringArray::from_slice(&[Some("red"), None, Some("green")]).into();
    let indices = [Some(2), None, Some(1), Some(0), Some(2)];
    let indexed =
        ColumnView::<String>::try_from(ColumnViewImpl::indexed(&indices, &values).unwrap())
            .unwrap();
    assert_eq!(
        (0..indexed.len())
            .map(|row| indexed.get(row))
            .collect::<Vec<_>>(),
        vec![Some("green"), None, None, Some("red"), Some("green")]
    );
}

#[test]
fn reads_expanded_families_through_array_constant_and_indexed_views() {
    let doubles: ArrayImpl = F64Array::from_slice(&[Some(-0.0), None, Some(f64::INFINITY)]).into();
    let double_indices = [Some(2), Some(0), None];
    let double_indexed =
        ColumnView::<f64>::try_from(ColumnViewImpl::indexed(&double_indices, &doubles).unwrap())
            .unwrap();
    assert_eq!(double_indexed.get(0), Some(f64::INFINITY));
    assert_eq!(
        double_indexed.get(1).unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(double_indexed.get(2), None);

    let boolean =
        ColumnView::<bool>::try_from(ColumnViewImpl::constant(ScalarRefImpl::Bool(true), 2))
            .unwrap();
    assert_eq!(boolean.get(0), Some(true));
    assert_eq!(boolean.get(1), Some(true));

    let smallints: ArrayImpl = I16Array::from_slice(&[Some(-1), None]).into();
    let smallints = ColumnView::<i16>::try_from(ColumnViewImpl::array(&smallints)).unwrap();
    assert_eq!(smallints.get(0), Some(-1));
    assert_eq!(smallints.get(1), None);

    let null = ColumnView::<f64>::try_from(ColumnViewImpl::null(PhysicalType::Float64, 2)).unwrap();
    assert_eq!(null.get(0), None);
    assert_eq!(null.get(1), None);

    let booleans: ArrayImpl = BoolArray::from_slice(&[Some(false)]).into();
    assert_eq!(booleans.physical_type(), PhysicalType::Bool);
}

#[test]
fn preserves_the_type_and_length_of_null_and_empty_views() {
    let null =
        ColumnView::<String>::try_from(ColumnViewImpl::null(PhysicalType::String, 2)).unwrap();
    assert_eq!(null.len(), 2);
    assert_eq!(null.get(0), None);
    assert_eq!(null.get(1), None);

    let empty = ColumnViewImpl::null(PhysicalType::Int32, 0);
    assert!(empty.is_empty());
    assert_eq!(empty.physical_type(), PhysicalType::Int32);

    let values: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    let no_indices: [Option<usize>; 0] = [];
    let indexed = ColumnViewImpl::indexed(&no_indices, &values).unwrap();
    assert!(indexed.is_empty());
}

#[test]
fn rejects_every_invalid_index_before_exposing_a_view() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    assert!(ColumnViewImpl::indexed(&[Some(0), Some(1)], &values).is_err());

    let empty_values: ArrayImpl = I32Array::from_slice(&[]).into();
    assert!(ColumnViewImpl::indexed(&[Some(0)], &empty_values).is_err());
}

#[test]
fn rejects_a_physical_type_mismatch_before_reading_rows() {
    let integers: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    assert!(ColumnView::<String>::try_from(ColumnViewImpl::array(&integers)).is_err());

    assert!(ColumnView::<String>::try_from(ColumnViewImpl::null(PhysicalType::Int32, 1)).is_err());
}
