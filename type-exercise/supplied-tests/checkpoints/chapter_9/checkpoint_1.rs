use std::any::Any;

use crate::{
    Array, ArrayImpl, ColumnViewImpl, Expression, I32Add, I32Array, PhysicalType,
    PrimitiveBinaryExpression, ScalarRefImpl,
};

fn assert_expression_bounds<T: Any + Send + Sync + ?Sized>() {}

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
