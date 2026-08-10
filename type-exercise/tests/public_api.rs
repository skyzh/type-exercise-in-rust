use type_exercise::{ArrayImpl, ColumnViewImpl, InvalidIndex};

fn assert_indexed_constructor(
    _: for<'a> fn(&'a [Option<usize>], &'a ArrayImpl) -> Result<ColumnViewImpl<'a>, InvalidIndex>,
) {
}

fn indexed_constructor<'a>(
    indices: &'a [Option<usize>],
    values: &'a ArrayImpl,
) -> Result<ColumnViewImpl<'a>, InvalidIndex> {
    ColumnViewImpl::indexed(indices, values)
}

fn compile_time_signature_probe() {
    assert_indexed_constructor(indexed_constructor);
}

#[test]
fn indexed_constructor_has_the_public_day_3_signature() {
    compile_time_signature_probe();
}
