use crate::{
    Array, ArrayImpl, ColumnViewImpl, PhysicalType, ScalarRefImpl, StringArray,
    build_builtin_expression,
};

#[test]
fn string_array_pins_bytes_offsets_and_validity() {
    let array = StringArray::from_slice(&[Some("rust"), None, Some(""), Some("类型")]);
    assert_eq!(array.data(), "rust类型".as_bytes());
    assert_eq!(array.offsets(), &[0, 4, 4, 4, 10]);
    assert_eq!(
        array.validity().iter().by_vals().collect::<Vec<_>>(),
        vec![true, false, true, true]
    );
    assert_eq!(
        array.iter().collect::<Vec<_>>(),
        vec![Some("rust"), None, Some(""), Some("类型")]
    );
}

#[test]
fn consumed_writer_is_the_only_non_null_publication_path() {
    let array = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/array/string_array.rs"
    ));
    let core = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/expression.rs"
    ));
    let scalar = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/string.rs"));
    assert!(array.contains("pub struct Writer<'a>"));
    assert!(array.contains("pub struct WriterUsed<'a>"));
    assert!(array.contains("pub fn write(self"));
    assert!(core.contains("Fn(&str, &str, crate::Writer<'a>) -> crate::WriterUsed<'a>"));
    assert!(scalar.contains("writer.write(|value|"));
    assert!(!scalar.contains("format!"));
}

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
