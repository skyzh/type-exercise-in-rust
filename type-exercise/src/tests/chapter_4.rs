use crate::{
    Array, ArrayImpl, BUILTIN_EXPRESSION_NAMES, BinaryExpression, BinaryScalarFunction,
    ColumnViewImpl, Expression, ExpressionError, I32Array, PhysicalType, ScalarRefImpl,
    StringArray, TypeMismatch, build_builtin_expression,
};

struct PanicOnCall;

impl BinaryScalarFunction for PanicOnCall {
    type Left = i32;
    type Right = i32;
    type Output = String;

    fn evaluate(&self, _left: i32, _right: i32) -> String {
        panic!("strict expressions must not evaluate a null row")
    }
}

struct StringLengthAdd;

impl BinaryScalarFunction for StringLengthAdd {
    type Left = String;
    type Right = i32;
    type Output = i32;

    fn evaluate(&self, left: &str, right: i32) -> i32 {
        i32::try_from(left.len()).unwrap().wrapping_add(right)
    }
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

    let expression: Box<dyn Expression> =
        Box::new(BinaryExpression::new("string_length_add", StringLengthAdd));
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::String, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::Int32);

    let strings: ArrayImpl = StringArray::from_slice(&[Some("rust"), None]).into();
    let inputs = [
        ColumnViewImpl::array(&strings),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
    ];
    let result = expression.evaluate(&inputs).unwrap();
    let result = <&I32Array>::try_from(&result).unwrap();
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![Some(6), None]);
}

#[test]
fn generates_the_complete_builtin_catalog() {
    assert_eq!(BUILTIN_EXPRESSION_NAMES, &["i32_add", "string_concat"]);
    for name in BUILTIN_EXPRESSION_NAMES {
        assert_eq!(build_builtin_expression(name).unwrap().name(), *name);
    }
    assert!(build_builtin_expression("add").is_none());
    assert!(build_builtin_expression("missing").is_none());
}

#[test]
fn concatenates_borrowed_dictionary_and_constant_strings() {
    let expression = build_builtin_expression("string_concat").unwrap();
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::String, PhysicalType::String]
    );
    assert_eq!(expression.output_type(), PhysicalType::String);

    let values: ArrayImpl = StringArray::from_slice(&[Some("rust"), None, Some("data")]).into();
    let keys = [Some(2), Some(0), None, Some(1)];
    let inputs = [
        ColumnViewImpl::dictionary(&keys, &values).unwrap(),
        ColumnViewImpl::constant(ScalarRefImpl::String("base"), 4),
    ];
    let result = expression.evaluate(&inputs).unwrap();
    let result = <&StringArray>::try_from(&result).unwrap();
    assert_eq!(
        result.iter().collect::<Vec<_>>(),
        vec![Some("database"), Some("rustbase"), None, None]
    );
}

#[test]
fn preserves_strict_nulls_through_the_erased_adapter() {
    let expression: Box<dyn Expression> =
        Box::new(BinaryExpression::new("panic_on_call", PanicOnCall));
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::Int32, PhysicalType::Int32]
    );
    assert_eq!(expression.output_type(), PhysicalType::String);
    let inputs = [
        ColumnViewImpl::null(PhysicalType::Int32, 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
    ];
    let result = expression.evaluate(&inputs).unwrap();
    let result = <&StringArray>::try_from(&result).unwrap();
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![None, None]);
}

#[test]
fn rejects_arity_before_indexing_or_converting_inputs() {
    let expression = build_builtin_expression("i32_add").unwrap();
    assert_eq!(
        expression.evaluate(&[]),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 0,
        })
    );

    let inputs = [ColumnViewImpl::null(PhysicalType::String, 1)];
    assert_eq!(
        expression.evaluate(&inputs),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 1,
        })
    );

    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
        ColumnViewImpl::null(PhysicalType::String, 1),
    ];
    assert_eq!(
        expression.evaluate(&inputs),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 3,
        })
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
    assert_eq!(
        expression.evaluate(&inputs),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }))
    );
}

#[test]
fn delegates_length_errors_to_the_typed_boundary() {
    let expression = build_builtin_expression("i32_add").unwrap();
    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
    ];
    assert_eq!(
        expression.evaluate(&inputs),
        Err(ExpressionError::InputLengthMismatch { left: 1, right: 2 })
    );
}
