use std::any::{Any, TypeId};

use crate::*;

fn assert_expression_bounds<T: Any + Send + Sync + ?Sized>() {}

pub(super) fn mixed_fixed_width_add_batch(
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
    let expression: Box<dyn Expression> =
        Box::new(PrimitiveBinaryExpression::new("any_probe", I32Add));
    assert_eq!(
        expression.as_ref().type_id(),
        TypeId::of::<PrimitiveBinaryExpression<I32Add>>()
    );
}
