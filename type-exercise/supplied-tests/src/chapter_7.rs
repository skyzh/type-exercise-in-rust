use crate::{
    Array, ArrayImpl, BatchExpression, BoolArray, ColumnViewImpl, Expression, I32Array,
    PhysicalType, auto_vectorize_binary,
};

fn add(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_binary::<i32, i32, i32, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        i32::wrapping_add,
    )
}

fn wrong_type(_inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    Ok(BoolArray::from_slice(&[Some(true), Some(false)]).into())
}

fn wrong_length(_inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    Ok(I32Array::from_slice(&[Some(1)]).into())
}

#[test]
fn erases_one_complete_typed_batch_behind_fixed_arity_metadata() {
    let expression = BatchExpression::new(
        "add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        add,
    );
    let erased: &dyn Expression = &expression;
    assert_eq!(erased.name(), "add");
    assert_eq!(erased.arity(), 2);
    assert_eq!(
        erased.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );

    let left: ArrayImpl = I32Array::from_slice(&[Some(1), None]).into();
    let right: ArrayImpl = I32Array::from_slice(&[Some(4), Some(8)]).into();
    let output = erased
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        I32Array::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(5), None]
    );
}

#[test]
fn validates_arity_type_length_and_output_before_returning() {
    let expression = BatchExpression::new(
        "add",
        [PhysicalType::Int32, PhysicalType::Int32],
        PhysicalType::Int32,
        add,
    );
    let values: ArrayImpl = I32Array::from_slice(&[Some(1), Some(2)]).into();
    assert!(expression.evaluate(&[]).is_err());
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&values),
                ColumnViewImpl::null(PhysicalType::Float64, 2),
            ])
            .is_err()
    );
    assert!(
        expression
            .evaluate(&[
                ColumnViewImpl::array(&values),
                ColumnViewImpl::null(PhysicalType::Int32, 1),
            ])
            .is_err()
    );

    let wrong_type_expression = BatchExpression::new(
        "wrong_type",
        [PhysicalType::Int32],
        PhysicalType::Int32,
        wrong_type,
    );
    assert!(
        wrong_type_expression
            .evaluate(&[ColumnViewImpl::array(&values)])
            .is_err()
    );

    let wrong_length_expression = BatchExpression::new(
        "wrong_length",
        [PhysicalType::Int32],
        PhysicalType::Int32,
        wrong_length,
    );
    assert!(
        wrong_length_expression
            .evaluate(&[ColumnViewImpl::array(&values)])
            .is_err()
    );
}
