use crate::{Array, ArrayImpl, I32Array, PHYSICAL_FAMILY_CATALOG, PhysicalFamily, StringArray};

#[test]
fn checkpoint_4_erases_arrays_through_the_two_row_catalog() {
    assert_eq!(
        PHYSICAL_FAMILY_CATALOG
            .iter()
            .filter(|entry| matches!(entry.family, PhysicalFamily::Int32 | PhysicalFamily::String))
            .map(|entry| (entry.family, entry.name))
            .collect::<Vec<_>>(),
        [
            (PhysicalFamily::Int32, "Int32"),
            (PhysicalFamily::String, "String"),
        ],
        "implement for_each_physical_family with the Int32 and String rows; Chapter 2 expands it"
    );

    let integers: ArrayImpl = I32Array::from_slice(&[Some(1), None]).into();
    assert_eq!(
        I32Array::try_from(integers)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(1), None]
    );

    let strings: ArrayImpl = StringArray::from_slice(&[Some("a"), Some("b")]).into();
    assert_eq!(
        <&StringArray>::try_from(&strings).unwrap().get(1),
        Some("b")
    );
    assert!(<&I32Array>::try_from(&strings).is_err());
}
