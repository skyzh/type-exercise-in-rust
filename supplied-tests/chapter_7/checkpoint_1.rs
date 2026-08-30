use crate::{
    Array, ArrayImpl, ColumnViewImpl, I32Array, Nullability, PhysicalType, ScalarRefImpl,
};

#[test]
fn checkpoint_1_classifies_physical_nullability() {
    let dense: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    assert_eq!(
        ColumnViewImpl::array(&dense).nullability(),
        Nullability::Nullable
    );
    assert_eq!(
        ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 2).nullability(),
        Nullability::NonNull
    );
    assert_eq!(
        ColumnViewImpl::null(PhysicalType::Int32, 2).nullability(),
        Nullability::Nullable
    );
    let indices = [0];
    assert_eq!(
        ColumnViewImpl::indexed(&indices, &dense)
            .unwrap()
            .nullability(),
        Nullability::Nullable
    );
}

#[test]
fn checkpoint_1_recovers_only_all_valid_fixed_width_arrays() {
    let nullable: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
    assert!(ColumnViewImpl::try_non_null_array(&nullable).is_none());

    let dense: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    assert_eq!(
        ColumnViewImpl::try_non_null_array(&dense)
            .unwrap()
            .nullability(),
        Nullability::NonNull
    );

    let built_dense: ArrayImpl = I32Array::from_slice(&[Some(4), Some(5)]).into();
    assert!(ColumnViewImpl::try_non_null_array(&built_dense).is_some());
    let empty: ArrayImpl = I32Array::from_values(Vec::new()).into();
    assert!(ColumnViewImpl::try_non_null_array(&empty).is_some());
}
