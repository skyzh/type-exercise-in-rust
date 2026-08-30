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
