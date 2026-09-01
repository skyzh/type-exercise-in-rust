use std::any::Any;

use crate::{
    Array, ArrayBuilder, ArrayImpl, BatchExpression, BinaryExpression, ColumnView, ColumnViewImpl,
    Expression, I32Add, I32Array, PhysicalType, PrimitiveBinaryExpression, ScalarRefImpl,
    StringArray,
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
fn checkpoint_1_evaluates_one_typed_expression_through_a_trait_object() {
    let expression: Box<dyn Expression> =
        Box::new(PrimitiveBinaryExpression::new("i32_add", I32Add));
    assert_eq!(expression.name(), "i32_add");
    assert_eq!(expression.arity(), 2);
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);

    let left: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
        ])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(12), None, Some(32)]
    );
}

#[test]
fn checkpoint_1_erased_expression_boundary_is_any_send_and_sync() {
    assert_expression_bounds::<dyn Expression>();
}

#[test]
fn checkpoint_2_rejects_a_kernel_result_with_wrong_metadata() {
    let expression = BinaryExpression::new(
        "mismatched_output",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Bool,
        mixed_fixed_width_add_batch,
    );
    let values: ArrayImpl = I32Array::from_values(vec![4]).into();
    let result = expression
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
        ]);
    assert!(result.is_err());
}

#[test]
fn checkpoint_2_preserves_strict_nulls_through_erasure() {
    let expression: Box<dyn Expression> = Box::new(BinaryExpression::new(
        "panic_on_call",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        panic_on_non_null_batch,
    ));
    let output = expression
        .evaluate(&[
            ColumnViewImpl::null(PhysicalType::Int32, 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ])
        .unwrap();
    assert_eq!(
        <&I32Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![None, None]
    );
}

#[test]
fn checkpoint_2_rejects_arity_before_indexing_inputs() {
    let expression: Box<dyn Expression> = Box::new(BinaryExpression::new(
        "mixed_fixed_width_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        mixed_fixed_width_add_batch,
    ));
    assert!(expression.evaluate(&[]).is_err());
    assert!(
        expression
            .evaluate(&[ColumnViewImpl::null(PhysicalType::Int32, 1)])
            .is_err()
    );
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
                ColumnViewImpl::null(PhysicalType::Int32, 1),
            ])
            .is_err()
    );
}

#[test]
fn checkpoint_2_delegates_type_errors_to_the_typed_boundary() {
    let expression: Box<dyn Expression> =
        Box::new(PrimitiveBinaryExpression::new("i32_add", I32Add));
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&strings),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
            ])
            .is_err()
    );
}

#[test]
fn checkpoint_2_delegates_length_errors_to_the_typed_boundary() {
    let expression: Box<dyn Expression> =
        Box::new(PrimitiveBinaryExpression::new("i32_add", I32Add));
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
            ])
            .is_err()
    );
}

#[test]
fn checkpoint_2_erases_unary_batch_metadata_and_rows() {
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
fn checkpoint_2_erases_binary_batch_metadata_and_rows() {
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
fn checkpoint_2_erases_ternary_batch_metadata_and_rows() {
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
