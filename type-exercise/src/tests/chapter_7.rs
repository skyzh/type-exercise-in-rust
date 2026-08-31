use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    ArithmeticOperator, Array, ArrayImpl, BinaryScalarFunction, ColumnViewImpl, I32Add, I32Array,
    PhysicalType, PrimitiveBinaryExpression, PrimitiveLoop, ScalarRefImpl, StringArray,
    auto_vectorize_binary, build_numeric_binary_expression,
};

fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

fn generic_add(inputs: &[ColumnViewImpl<'_>; 2]) -> ArrayImpl {
    auto_vectorize_binary::<i32, i32, i32, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        i32::wrapping_add,
    )
    .unwrap()
}

#[test]
fn selects_all_raw_shapes_including_nullable_arrays_and_typed_nulls() {
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
        let generic = generic_add(&inputs);
        let (output, selected) = expression.evaluate_with_loop(&inputs).unwrap();
        assert_eq!(selected, expected_loop);
        assert_eq!(i32_values(&output), expected_values);
        assert_eq!(i32_values(&output), i32_values(&generic));
    }
}

#[test]
fn combines_nullable_validity_by_storage_word() {
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
}

struct CountingAdd {
    calls: Arc<AtomicUsize>,
}

impl BinaryScalarFunction for CountingAdd {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate(&self, left: i32, right: i32) -> i32 {
        self.calls.fetch_add(1, Ordering::SeqCst);
        left.wrapping_add(right)
    }
}

#[test]
fn constant_constant_invokes_the_scalar_once_or_not_at_all() {
    let calls = Arc::new(AtomicUsize::new(0));
    let expression = PrimitiveBinaryExpression::new(
        "counting_add",
        CountingAdd {
            calls: Arc::clone(&calls),
        },
    );
    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 65),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 65),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::ConstantConstant);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(i32_values(&output), vec![Some(7); 65]);

    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 0),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 0),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::ConstantConstant);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(i32_values(&output).is_empty());
}

#[test]
fn indexed_inputs_use_only_the_gather_fallback() {
    let dictionary_values: ArrayImpl = I32Array::from_slice(&[Some(4), Some(8), None]).into();
    let keys = [1, 2, 0];
    let right: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let inputs = [
        ColumnViewImpl::indexed(&keys, &dictionary_values).unwrap(),
        ColumnViewImpl::array(&right),
    ];
    let generic = generic_add(&inputs);
    let (output, selected) = PrimitiveBinaryExpression::new("i32_add", I32Add)
        .evaluate_with_loop(&inputs)
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::Indexed);
    assert_eq!(i32_values(&output), vec![Some(9), None, Some(7)]);
    assert_eq!(i32_values(&output), i32_values(&generic));
}

#[test]
fn fallible_division_never_observes_a_null_hidden_zero() {
    let expression = build_numeric_binary_expression(
        "numeric_divide",
        ArithmeticOperator::Divide,
        PhysicalType::Int32,
        PhysicalType::Int32,
        PhysicalType::Int32,
    );
    let hidden_zero: ArrayImpl = I32Array::from_slice(&[None, Some(2)]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(8), 2),
            ColumnViewImpl::array(&hidden_zero),
        ])
        .unwrap();
    assert_eq!(i32_values(&output), vec![None, Some(4)]);

    let visible_zero: ArrayImpl = I32Array::from_slice(&[None, Some(0)]).into();
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(8), 2),
                ColumnViewImpl::array(&visible_zero),
            ])
            .unwrap_err()
            .to_string(),
        "function `numeric_divide` failed at row 1: division by zero"
    );
}

struct WrappingSubtract;

impl BinaryScalarFunction for WrappingSubtract {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate(&self, left: i32, right: i32) -> i32 {
        left.wrapping_sub(right)
    }
}

#[test]
fn raw_loops_are_branch_free_and_preserve_operand_order() {
    let source = include_str!("../expression.rs");
    for helper in [
        "fn raw_array_array",
        "fn raw_array_constant",
        "fn raw_constant_array",
    ] {
        let body = source
            .split(helper)
            .nth(1)
            .unwrap()
            .split("\nfn ")
            .next()
            .unwrap();
        for forbidden in [".get(", "Option", "validity", "match ", "if "] {
            assert!(!body.contains(forbidden), "{helper} contains {forbidden}");
        }
    }

    let values: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let expression = PrimitiveBinaryExpression::new("subtract", WrappingSubtract);
    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 3),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::ArrayConstant);
    assert_eq!(i32_values(&output), vec![Some(7), None, Some(27)]);

    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 3),
            ColumnViewImpl::array(&values),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::ConstantArray);
    assert_eq!(i32_values(&output), vec![Some(-7), None, Some(-27)]);
}

#[test]
fn preserves_runtime_type_arity_and_length_errors() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let integers: ArrayImpl = I32Array::from_values(vec![1, 2]).into();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();

    assert_eq!(
        expression
            .evaluate(&[ColumnViewImpl::array(&integers)])
            .unwrap_err()
            .to_string(),
        "input arity mismatch: expected 2, got 1"
    );
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&strings),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
            ])
            .unwrap_err()
            .to_string(),
        "input 0 type mismatch: expected Int32, got String"
    );
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&integers),
                ColumnViewImpl::array(&strings),
            ])
            .unwrap_err()
            .to_string(),
        "input 1 type mismatch: expected Int32, got String"
    );
    assert_eq!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&integers),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
            ])
            .unwrap_err()
            .to_string(),
        "input 1 length mismatch: expected 2, got 1"
    );
}
