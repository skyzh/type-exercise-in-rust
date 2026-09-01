#[test]
fn checkpoint_1_binds_raw_i32_arrays_and_constants() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/column.rs"
    ));

    for required in [
        "enum RawI32Column<'a>",
        "Array {",
        "values: &'a [i32]",
        "validity: &'a BitVec",
        "Constant {",
        "value: i32",
        "valid: bool",
        "len: usize",
        "fn as_raw_i32(&self)",
    ] {
        assert!(source.contains(required), "missing raw binding: {required}");
    }
    assert!(!source.contains("pub enum RawI32Column"));
    assert!(!source.contains("pub fn as_raw_i32"));
    assert!(!source.contains("pub fn len(self)"));
    for forbidden in [
        "enum Nullability",
        "try_non_null_array",
        "fn nullability(&self)",
    ] {
        assert!(
            !source.contains(forbidden),
            "Day 7 must not expose stale nullability API: {forbidden}"
        );
    }
}

#[test]
fn checkpoint_1_keeps_indexed_detection_separate() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/column.rs"
    ));

    let raw_body = source
        .split("fn as_raw_i32(&self)")
        .nth(1)
        .expect("as_raw_i32 body")
        .split("fn is_indexed")
        .next()
        .expect("as_raw_i32 boundary");
    assert!(raw_body.contains("ColumnViewImplKind::Array(ArrayImpl::Int32(array))"));
    assert!(raw_body.contains("Some(ScalarRefImpl::Int32(value))"));
    assert!(raw_body.contains("None => 0"));
    assert!(raw_body.contains("valid: value.is_some()"));
    assert!(raw_body.contains("_ => None"));

    let indexed_body = source
        .split("fn is_indexed")
        .nth(1)
        .expect("is_indexed body")
        .split("\n    }")
        .next()
        .expect("is_indexed boundary");
    assert!(
        indexed_body.contains("matches!(self.kind, ColumnViewImplKind::Indexed { .. })")
            || indexed_body.contains("matches!(&self.kind, ColumnViewImplKind::Indexed { .. })")
    );
}
