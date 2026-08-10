//! Broken learner-compatibility fixture.
//!
//! This mirrors exactly what a copied supplied test must never do: name or
//! match a `BindError`/`ExpressionError` variant. It must NOT compile against
//! the opaque error layout; `cargo x check-opaque-compat` enforces that.

use type_exercise::{BoundExpression, ColumnViewImpl, DataType, FunctionRegistry};

pub fn leak_variant(registry: &FunctionRegistry) -> Result<BoundExpression, type_exercise::BindError> {
    // This pins the reference error layout and must fail under `opaque-errors`.
    let _ = type_exercise::BindError::UnknownFunction {
        name: "missing".to_owned(),
    };
    let _ = ColumnViewImpl::constant(type_exercise::ScalarRefImpl::Int32(1), 1);
    registry.bind_binary("+", DataType::Integer, DataType::Integer)
}
