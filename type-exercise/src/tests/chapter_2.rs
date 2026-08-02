use crate::{
    Array, ArrayImpl, ColumnView, ColumnViewImpl, I32Array, InvalidDictionaryKey, PhysicalType,
    ScalarRefImpl, StringArray, TypeMismatch,
};

#[test]
fn reads_arrays_constants_and_dictionaries_as_logical_rows() {
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
    let keys = [Some(2), None, Some(1), Some(0)];
    let dictionary =
        ColumnView::<String>::try_from(ColumnViewImpl::dictionary(&keys, &values).unwrap())
            .unwrap();
    assert_eq!(
        (0..dictionary.len())
            .map(|row| dictionary.get(row))
            .collect::<Vec<_>>(),
        vec![Some("green"), None, None, Some("red")]
    );
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
    let no_keys: [Option<usize>; 0] = [];
    let dictionary = ColumnViewImpl::dictionary(&no_keys, &values).unwrap();
    assert!(dictionary.is_empty());
}

#[test]
fn rejects_every_invalid_dictionary_key_with_a_precise_error() {
    let values: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    assert_eq!(
        ColumnViewImpl::dictionary(&[Some(0), Some(1)], &values),
        Err(InvalidDictionaryKey {
            row: 1,
            key: 1,
            dictionary_len: 1,
        })
    );

    let empty_values: ArrayImpl = I32Array::from_slice(&[]).into();
    assert_eq!(
        ColumnViewImpl::dictionary(&[Some(0)], &empty_values),
        Err(InvalidDictionaryKey {
            row: 0,
            key: 0,
            dictionary_len: 0,
        })
    );
}

#[test]
fn rejects_a_physical_type_mismatch_before_reading_rows() {
    let integers: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    assert_eq!(
        ColumnView::<String>::try_from(ColumnViewImpl::array(&integers)).unwrap_err(),
        TypeMismatch {
            expected: PhysicalType::String,
            actual: PhysicalType::Int32,
        }
    );

    assert_eq!(
        ColumnView::<String>::try_from(ColumnViewImpl::null(PhysicalType::Int32, 1)).unwrap_err(),
        TypeMismatch {
            expected: PhysicalType::String,
            actual: PhysicalType::Int32,
        }
    );
}
