//! Clean learner-compatibility fixture.
//!
//! This mirrors what a copied supplied test may legitimately do against the
//! learner API: construct expressions through the public builders and assert
//! behavioral `Err` without ever naming a `BindError` or `ExpressionError`
//! variant. It must compile against the opaque error layout; `cargo x
//! check-opaque-compat` enforces that.

use type_exercise::{
    Array, BoundExpression, ColumnViewImpl, DataType, FunctionRegistry, I32Array, ScalarRefImpl,
};

pub fn bind_and_evaluate(registry: &FunctionRegistry) -> Option<i32> {
    let expression: BoundExpression = registry
        .bind_binary("+", DataType::Integer, DataType::Integer)
        .ok()?;
    let output = expression
        .evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int32(9), 1),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 1),
        ])
        .ok()?;
    let array = <&I32Array>::try_from(&output).ok()?;
    array.get(0)
}
