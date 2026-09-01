use crate::{
    Array, ArrayImpl, ColumnViewImpl, I32Add, I32Array, PrimitiveBinaryExpression, PrimitiveLoop,
    PhysicalType, ScalarRefImpl,
};

fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

#[test]
fn checkpoint_1_binds_raw_i32_arrays_and_constants() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/column.rs"
    ));
    for required in [
        "enum RawI32Column<'a>",
        "values: &'a [i32]",
        "validity: &'a BitVec",
        "value: i32",
        "valid: bool",
        "valid: value.is_some()",
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
    assert!(raw_body.contains("_ => None"));
    let indexed_body = source
        .split("fn is_indexed")
        .nth(1)
        .expect("is_indexed body")
        .split("\n    }")
        .next()
        .expect("is_indexed boundary");
    assert!(indexed_body.contains("matches!(self.kind, ColumnViewImplKind::Indexed { .. })"));
}

#[test]
fn checkpoint_2_selects_all_four_raw_shapes() {
    let core_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/expression.rs"
    ));
    let facade_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../expr/src/numeric.rs"
    ));
    assert!(core_source.contains("pub trait Expression: Send + Sync"));
    for forbidden in [
        "Expression: Any",
        "fn name(&self)",
        "fn input_types(&self)",
        "fn output_type(&self)",
        "output_nullability",
    ] {
        assert!(
            !core_source.contains(forbidden),
            "Chapter 9 metadata leaked into Day 7 core: {forbidden}"
        );
        assert!(
            !facade_source.contains(forbidden),
            "Chapter 9 metadata leaked into Day 7 facade: {forbidden}"
        );
    }

    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let left: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), None]).into();
    let cases = [
        (
            [ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)],
            PrimitiveLoop::ArrayArray,
            vec![Some(11), None, None],
        ),
        (
            [
                ColumnViewImpl::array(&left),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
            ],
            PrimitiveLoop::ArrayConstant,
            vec![Some(15), None, Some(35)],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
                ColumnViewImpl::array(&right),
            ],
            PrimitiveLoop::ConstantArray,
            vec![Some(6), Some(7), None],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 3),
            ],
            PrimitiveLoop::ConstantConstant,
            vec![Some(12), Some(12), Some(12)],
        ),
        (
            [
                ColumnViewImpl::array(&left),
                ColumnViewImpl::null(PhysicalType::Int32, 3),
            ],
            PrimitiveLoop::ArrayConstant,
            vec![None, None, None],
        ),
    ];

    for (inputs, expected_loop, expected_values) in cases {
        let (output, selected) = expression.evaluate_with_loop(&inputs).unwrap();
        assert_eq!(selected, expected_loop);
        assert_eq!(i32_values(&output), expected_values);
    }
}

#[test]
fn checkpoint_2_combines_validity_by_storage_word_and_falls_back_for_indexed() {
    let left_values = (0..137)
        .map(|row| (row % 3 != 0).then_some(row))
        .collect::<Vec<_>>();
    let right_values = (0..137)
        .map(|row| (row % 5 != 0).then_some(1000 + row))
        .collect::<Vec<_>>();
    let left: ArrayImpl = I32Array::from_slice(&left_values).into();
    let right: ArrayImpl = I32Array::from_slice(&right_values).into();
    let (output, selected) = PrimitiveBinaryExpression::new("i32_add", I32Add)
        .evaluate_with_loop(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::ArrayArray);
    let expected = left_values
        .iter()
        .zip(&right_values)
        .map(|(left, right)| {
            left.zip(*right)
                .map(|(left, right)| left.wrapping_add(right))
        })
        .collect::<Vec<_>>();
    assert_eq!(i32_values(&output), expected);
    assert!(expected.len() > usize::BITS as usize * 2);

    let dictionary: ArrayImpl = I32Array::from_slice(&[Some(4), None, Some(8)]).into();
    let keys = [2, 1, 0];
    let one: ArrayImpl = I32Array::from_values(vec![1, 1, 1]).into();
    let (output, selected) = PrimitiveBinaryExpression::new("i32_add", I32Add)
        .evaluate_with_loop(&[
            ColumnViewImpl::indexed(&keys, &dictionary).unwrap(),
            ColumnViewImpl::array(&one),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::Indexed);
    assert_eq!(i32_values(&output), vec![Some(9), None, Some(5)]);
}
