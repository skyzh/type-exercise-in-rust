use std::any::Any;

use crate::{
    Array, ArrayBuilder, ArrayImpl, BUILTIN_EXPRESSION_NAMES, BatchExpression, BinaryExpression,
    BoolArray, BooleanOperator, ColumnView, ColumnViewImpl, Expression, I32Array, PhysicalType,
    ScalarRefImpl, StringArray, build_boolean_expression, build_builtin_expression,
};

fn assert_expression_bounds<T: Any + Send + Sync + ?Sized>() {}

fn panic_on_non_null_batch(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let right = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = left
            .get(row)
            .zip(right.get(row))
            .map(|_| panic!("strict expressions must not evaluate a null row"));
        output.push(value);
    }
    Ok(output.finish().into())
}

fn mixed_fixed_width_add_batch(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let right = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        output.push(
            left.get(row)
                .zip(right.get(row))
                .map(|(left, right)| left.wrapping_add(right)),
        );
    }
    Ok(output.finish().into())
}

#[test]
fn evaluates_a_builtin_through_a_trait_object() {
    let expression: Box<dyn Expression> = build_builtin_expression("i32_add").unwrap();
    assert_eq!(expression.name(), "i32_add");
    assert_eq!(expression.arity(), 2);
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);

    let left: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let inputs = [
        ColumnViewImpl::array(&left),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
    ];
    let result = expression.evaluate(&inputs).unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![Some(12), None, Some(32)]
    );

    let expression: Box<dyn Expression> = Box::new(BinaryExpression::new(
        "mixed_fixed_width_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        mixed_fixed_width_add_batch,
    ));
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);

    let values: ArrayImpl = I32Array::from_slice(&[Some(4), None]).into();
    let inputs = [
        ColumnViewImpl::array(&values),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
    ];
    let result = expression.evaluate(&inputs).unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![Some(6), None]);
}

#[test]
fn erased_expression_boundary_is_any_send_and_sync() {
    assert_expression_bounds::<dyn Expression>();
}

#[test]
fn generates_the_complete_builtin_catalog() {
    assert!(BUILTIN_EXPRESSION_NAMES.contains(&"i32_add"));
    for name in BUILTIN_EXPRESSION_NAMES {
        assert_eq!(build_builtin_expression(name).unwrap().name(), *name);
    }
    assert!(build_builtin_expression("add").is_none());
    assert!(build_builtin_expression("missing").is_none());
}

#[test]
fn rejects_a_kernel_result_that_disagrees_with_declared_metadata() {
    let expression = BinaryExpression::new(
        "mismatched_output",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Bool,
        mixed_fixed_width_add_batch,
    );
    let values: ArrayImpl = I32Array::from_values(vec![4]).into();
    let error = expression
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
        ])
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "output type mismatch: expected Bool, got Int32"
    );
}

#[test]
fn preserves_strict_nulls_through_the_erased_adapter() {
    let expression: Box<dyn Expression> = Box::new(BinaryExpression::new(
        "panic_on_call",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        panic_on_non_null_batch,
    ));
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);
    let inputs = [
        ColumnViewImpl::null(PhysicalType::Int32, 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
    ];
    let result = expression.evaluate(&inputs).unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![None, None]);
}

#[test]
fn rejects_arity_before_indexing_or_converting_inputs() {
    let expression: Box<dyn Expression> = Box::new(BinaryExpression::new(
        "mixed_fixed_width_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        mixed_fixed_width_add_batch,
    ));
    assert!(expression.evaluate(&[]).is_err());

    let inputs = [ColumnViewImpl::null(PhysicalType::Int32, 1)];
    assert!(expression.evaluate(&inputs).is_err());

    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
        ColumnViewImpl::null(PhysicalType::Int32, 1),
    ];
    assert!(expression.evaluate(&inputs).is_err());
}

#[test]
fn delegates_physical_type_errors_to_the_typed_boundary() {
    let expression = build_builtin_expression("i32_add").unwrap();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    let inputs = [
        ColumnViewImpl::array(&strings),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
    ];
    assert!(expression.evaluate(&inputs).is_err());
}

#[test]
fn delegates_length_errors_to_the_typed_boundary() {
    let expression = build_builtin_expression("i32_add").unwrap();
    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
    ];
    assert!(expression.evaluate(&inputs).is_err());
}

#[test]
fn i32_builtin_uses_the_shared_binary_auto_vectorizer() {
    let binder = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../expr/src/binder.rs"
    ));
    assert!(binder.contains("PrimitiveBinaryExpression::new(\"i32_add\", I32Add)"));
    let core = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/src/expression.rs"
    ));
    assert!(!core.contains("evaluate_i32_add_batch"));
    assert!(core.contains("impl<F> PrimitiveBinaryExpression<F>"));
}

#[test]
fn boolean_expressions_delegate_through_the_erased_boundary() {
    // Day 7 deferred ledger: the three-valued Boolean expression must reach
    // the same rows and metadata through dyn Expression as through its
    // inherent evaluate.
    let and: Box<dyn Expression> = Box::new(build_boolean_expression(BooleanOperator::And));
    assert_eq!(and.name(), "boolean_and");
    assert_eq!(and.arity(), 2);
    assert_eq!(and.input_types(), &[PhysicalType::Bool, PhysicalType::Bool]);
    assert_eq!(and.output_type(), PhysicalType::Bool);

    let left: ArrayImpl = BoolArray::from_slice(&[Some(true), Some(false), None]).into();
    let right: ArrayImpl = BoolArray::from_slice(&[Some(false), Some(true), Some(false)]).into();
    let result = and
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&result)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(false), Some(false)]
    );

    let not: Box<dyn Expression> = Box::new(build_boolean_expression(BooleanOperator::Not));
    assert_eq!(not.name(), "boolean_not");
    assert_eq!(not.arity(), 1);
    assert_eq!(not.input_types(), &[PhysicalType::Bool]);
    let result = not.evaluate(&[ColumnViewImpl::array(&left)]).unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&result)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(true), None]
    );
}

fn checked_neg_batch(
    _expression: &BatchExpression<1>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    let input = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(input.len());
    for row in 0..input.len() {
        output.push(input.get(row).map(i32::wrapping_neg));
    }
    Ok(output.finish().into())
}

fn checked_add_batch(
    _expression: &BatchExpression<2>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let right = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        output.push(
            left.get(row)
                .zip(right.get(row))
                .map(|(left, right)| left.wrapping_add(right)),
        );
    }
    Ok(output.finish().into())
}

fn checked_clamp_batch(
    _expression: &BatchExpression<3>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    let value = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let lower = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let upper = ColumnView::<i32>::try_from(inputs[2].clone())?;
    let mut output = <I32Array as Array>::Builder::with_capacity(value.len());
    for row in 0..value.len() {
        output.push(
            value
                .get(row)
                .zip(lower.get(row))
                .zip(upper.get(row))
                .map(|((value, lower), upper)| value.clamp(lower, upper)),
        );
    }
    Ok(output.finish().into())
}

#[test]
fn erased_unary_batch_delegates_metadata_and_rows() {
    let expression: Box<dyn Expression> = Box::new(BatchExpression::new(
        "checked_neg",
        [PhysicalType::Int32],
        PhysicalType::Int32,
        checked_neg_batch,
    ));
    assert_eq!(expression.name(), "checked_neg");
    assert_eq!(expression.arity(), 1);
    assert_eq!(expression.input_types(), &[PhysicalType::Int32]);
    assert_eq!(expression.output_type(), PhysicalType::Int32);

    let output = expression
        .evaluate(&[ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 2)])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(-7), Some(-7)]
    );
}

#[test]
fn erased_binary_batch_delegates_metadata_and_rows() {
    let expression: Box<dyn Expression> = Box::new(BatchExpression::new(
        "checked_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        checked_add_batch,
    ));
    assert_eq!(expression.name(), "checked_add");
    assert_eq!(expression.arity(), 2);
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);

    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
        ])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(3), Some(3)]
    );
}

#[test]
fn erased_ternary_batch_delegates_metadata_and_rows() {
    let expression: Box<dyn Expression> = Box::new(BatchExpression::new(
        "checked_clamp",
        [
            PhysicalType::Int32,
            PhysicalType::Int32,
            PhysicalType::Int32,
        ],
        PhysicalType::Int32,
        checked_clamp_batch,
    ));
    assert_eq!(expression.name(), "checked_clamp");
    assert_eq!(expression.arity(), 3);
    assert_eq!(
        expression.input_types(),
        &[
            PhysicalType::Int32,
            PhysicalType::Int32,
            PhysicalType::Int32
        ]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);

    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(25), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(20), 1),
        ])
        .unwrap();
    assert_eq!(<&I32Array>::try_from(&output).unwrap().get(0), Some(20));
}
