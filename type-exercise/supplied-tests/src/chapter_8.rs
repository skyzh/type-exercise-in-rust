use crate::{
    Array, ArrayImpl, BoolArray, ColumnViewImpl, DataType, FunctionRegistry, I16Array, I32Array,
    ScalarRefImpl, StringArray,
};

#[test]
fn binds_logical_numeric_calls_once_then_evaluates_the_selected_batch() {
    let registry = FunctionRegistry::with_builtins();
    let expression = registry
        .bind("+", &[DataType::SmallInt, DataType::Integer])
        .unwrap();
    assert_eq!(expression.output_type(), DataType::Integer);
    assert_eq!(expression.physical_name(), "numeric_add");

    let left: ArrayImpl = I16Array::from_slice(&[Some(2), None]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&left),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 2),
        ])
        .unwrap();
    assert_eq!(
        I32Array::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(7), None]
    );
}

#[test]
fn keeps_boolean_and_string_semantics_behind_the_same_registry() {
    let registry = FunctionRegistry::with_builtins();
    let and = registry
        .bind("boolean_and", &[DataType::Boolean, DataType::Boolean])
        .unwrap();
    assert_eq!(and.physical_name(), "boolean_and");
    let left: ArrayImpl = BoolArray::from_slice(&[Some(false), Some(true)]).into();
    let output = and
        .evaluate(&[
            ColumnViewImpl::array(&left),
            ColumnViewImpl::null(crate::PhysicalType::Bool, 2),
        ])
        .unwrap();
    assert_eq!(
        BoolArray::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), None]
    );

    let concat = registry
        .bind("concat", &[DataType::Varchar, DataType::Varchar])
        .unwrap();
    assert_eq!(concat.physical_name(), "string_concat");
    let words: ArrayImpl = StringArray::from_slice(&[Some("data"), None]).into();
    let output = concat
        .evaluate(&[
            ColumnViewImpl::array(&words),
            ColumnViewImpl::constant(ScalarRefImpl::String("base"), 2),
        ])
        .unwrap();
    assert_eq!(
        StringArray::try_from(output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some("database"), None]
    );
}

#[test]
fn rejects_unknown_arity_and_lossy_signatures_before_evaluation() {
    let registry = FunctionRegistry::with_builtins();
    assert!(registry.bind("missing", &[DataType::Integer]).is_err());
    assert!(registry.bind("+", &[DataType::Integer]).is_err());
    assert!(
        registry
            .bind("+", &[DataType::BigInt, DataType::Double])
            .is_err()
    );
}
