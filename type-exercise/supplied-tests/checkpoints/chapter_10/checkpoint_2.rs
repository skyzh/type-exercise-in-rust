use crate::{Array, StringArray};

#[test]
fn checkpoint_1_string_array_pins_bytes_offsets_and_validity() {
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
fn checkpoint_2_consumed_writer_is_the_only_publication_path() {
    let array = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/array/string_array.rs"
    ));
    let core = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/expression.rs"
    ));
    assert!(array.contains("pub struct Writer<'a>"));
    assert!(array.contains("pub struct WriterUsed<'a>"));
    assert!(array.contains("pub fn write(self"));
    assert!(core.contains("Fn(&str, &str, crate::Writer<'a>) -> crate::WriterUsed<'a>"));
}
