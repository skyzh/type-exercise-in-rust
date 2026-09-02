use super::checkpoint_1::mixed_fixed_width_add_batch;
use crate::*;

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

fn mismatched_batch_output(
    _expression: &BatchExpression<2>,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    mixed_fixed_width_add_batch(inputs)
}

fn panic_if_generic_kernel_is_called(
    _expression: &BatchExpression<2>,
    _inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl> {
    panic!("invalid inputs must be rejected before the batch kernel")
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
    let result = expression.evaluate(&[
        ColumnViewImpl::array(&values),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
    ]);
    assert!(result.is_err());

    let expression = BatchExpression::new(
        "mismatched_batch_output",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Bool,
        mismatched_batch_output,
    );
    let result = expression.evaluate(&[
        ColumnViewImpl::array(&values),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
    ]);
    assert!(result.is_err());
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
fn generic_adapter_validates_before_calling_its_kernel() {
    let expression = BatchExpression::new(
        "validation_probe",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        panic_if_generic_kernel_is_called,
    );
    assert!(expression.evaluate(&[]).is_err());

    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&strings),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
            ])
            .is_err()
    );

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
