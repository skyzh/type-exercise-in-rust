use crate::*;

fn write_pair<'a>(left: &str, right: &str, writer: Writer<'a>) -> WriterUsed<'a> {
    writer.write(|value| {
        value.push_str(left);
        value.push_str(right);
    })
}

#[test]
fn writer_callback_builds_visible_output_and_preserves_nulls() {
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
