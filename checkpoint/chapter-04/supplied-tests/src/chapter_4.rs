use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnViewImpl, ScalarRefImpl, StringArray, StringArrayBuilder,
    evaluate_writer_binary,
};

#[test]
fn publishes_one_variable_width_row_per_consumed_writer() {
    let left: ArrayImpl = StringArray::from_slice(&[Some("vec"), None, Some("")]).into();
    let right = ColumnViewImpl::constant(ScalarRefImpl::String("tor"), 3);
    let output = evaluate_writer_binary(
        ColumnViewImpl::array(&left),
        right,
        |left, right, writer| {
            writer.write(|value| {
                value.push_str(left);
                value.push_str(right);
            })
        },
    )
    .unwrap();
    let output = StringArray::try_from(output).unwrap();
    assert_eq!(output.data(), b"vectortor");
    assert_eq!(output.offsets(), &[0, 6, 6, 9]);
    assert_eq!(
        output.iter().collect::<Vec<_>>(),
        vec![Some("vector"), None, Some("tor")]
    );
}

#[test]
fn rolls_back_failed_variable_width_rows() {
    let mut builder = StringArrayBuilder::with_capacity(2);
    let result = builder.try_push_with(|value| {
        value.push_str("partial");
        Err::<(), _>("stop")
    });
    assert_eq!(result, Err("stop"));
    builder.push(Some("kept"));
    let array = builder.finish();
    assert_eq!(array.data(), b"kept");
    assert_eq!(array.offsets(), &[0, 4]);
    assert_eq!(array.iter().collect::<Vec<_>>(), vec![Some("kept")]);
}
