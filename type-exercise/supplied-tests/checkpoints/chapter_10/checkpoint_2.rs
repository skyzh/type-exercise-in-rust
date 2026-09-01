use crate::{
    Array, ArrayImpl, ColumnViewImpl, ScalarRefImpl, StringArray, Writer, WriterUsed,
    evaluate_writer_binary,
};

fn write_pair<'a>(left: &str, right: &str, writer: Writer<'a>) -> WriterUsed<'a> {
    writer.write(|value| {
        value.push_str(left);
        value.push_str(right);
    })
}

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
fn checkpoint_2_writer_callback_builds_visible_output_and_preserves_nulls() {
    let left: ArrayImpl = StringArray::from_slice(&[Some("rust"), None]).into();
    let output = evaluate_writer_binary(
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::String("ace"), 2),
        write_pair,
    )
    .unwrap();
    assert_eq!(
        <&StringArray>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some("rustace"), None]
    );
}
