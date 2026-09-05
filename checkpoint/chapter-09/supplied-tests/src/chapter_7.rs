use std::sync::atomic::{AtomicUsize, Ordering};

use crate::*;

fn add_kernel(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_binary::<i32, i32, i32, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        i32::wrapping_add,
    )
}

fn i32_values(array: ArrayImpl) -> Vec<Option<i32>> {
    I32Array::try_from(array).unwrap().iter().collect()
}

#[test]
fn fixed_arity_expression_exposes_and_evaluates_its_physical_contract() {
    let kernel: BatchKernel = add_kernel;
    let expression = BatchExpression::new(
        "i32_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        kernel,
    );
    assert_eq!(expression.name(), "i32_add");
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);

    let left: ArrayImpl = I32Array::from_slice(&[Some(2), None, Some(7)]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
        ])
        .unwrap();
    assert_eq!(i32_values(output), vec![Some(7), None, Some(12)]);
}

#[test]
fn fixed_arity_expression_keeps_the_same_behavior_after_erasure() {
    let expression = BatchExpression::new(
        "i32_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        add_kernel,
    );

    let left: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2), Some(3)]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(10), None, Some(30)]).into();
    let inputs = [ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)];
    let direct = i32_values(expression.evaluate(&inputs).unwrap());

    let expression: Box<dyn Expression> = Box::new(expression);
    assert_eq!(expression.name(), "i32_add");
    assert_eq!(expression.arity(), 2);
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);
    let erased = i32_values(expression.evaluate(&inputs).unwrap());
    assert_eq!(erased, direct);
    assert_eq!(erased, vec![Some(11), None, Some(33)]);
}

static KERNEL_CALLS: AtomicUsize = AtomicUsize::new(0);

fn counting_kernel(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    KERNEL_CALLS.fetch_add(1, Ordering::SeqCst);
    add_kernel(inputs)
}

fn wrong_type_kernel(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    Ok(BoolArray::from_slice(&vec![Some(true); inputs[0].len()]).into())
}

fn wrong_length_kernel(_inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    Ok(I32Array::from_slice(&[Some(1)]).into())
}

#[test]
fn validates_inputs_before_the_kernel_and_checks_the_returned_batch() {
    KERNEL_CALLS.store(0, Ordering::SeqCst);
    let expression = BatchExpression::new(
        "counting_add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        counting_kernel,
    );
    let integers: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2)]).into();
    let short: ArrayImpl = I32Array::from_slice(&[Some(1)]).into();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong"), Some("type")]).into();

    assert!(
        expression
            .evaluate(&[ColumnViewImpl::array(&integers)])
            .is_err()
    );
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&strings),
                ColumnViewImpl::array(&integers),
            ])
            .is_err()
    );
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&integers),
                ColumnViewImpl::array(&short),
            ])
            .is_err()
    );
    assert_eq!(KERNEL_CALLS.load(Ordering::SeqCst), 0);

    let inputs = [
        ColumnViewImpl::array(&integers),
        ColumnViewImpl::array(&integers),
    ];
    assert!(
        BatchExpression::new(
            "wrong_type",
            [PhysicalType::Int32, PhysicalType::Int32],
            PhysicalType::Int32,
            wrong_type_kernel,
        )
        .evaluate(&inputs)
        .is_err()
    );
    assert!(
        BatchExpression::new(
            "wrong_length",
            [PhysicalType::Int32, PhysicalType::Int32],
            PhysicalType::Int32,
            wrong_length_kernel,
        )
        .evaluate(&inputs)
        .is_err()
    );
}
