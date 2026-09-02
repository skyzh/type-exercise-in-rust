use crate::*;

#[test]
fn concatenates_borrowed_indexed_and_constant_strings() {
    let expression = build_builtin_expression("string_concat").unwrap();
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::String, PhysicalType::String]
    );
    assert_eq!(expression.output_type(), PhysicalType::String);

    let values: ArrayImpl = StringArray::from_slice(&[Some("rust"), None, Some("data")]).into();
    let keys = [2, 0, 1, 1];
    let result = expression
        .evaluate(&[
            ColumnViewImpl::indexed(&keys, &values).unwrap(),
            ColumnViewImpl::constant(ScalarRefImpl::String("base"), 4),
        ])
        .unwrap();
    let result = <&StringArray>::try_from(&result).unwrap();
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![Some("database"), Some("rustbase"), None, None]
    );
}
