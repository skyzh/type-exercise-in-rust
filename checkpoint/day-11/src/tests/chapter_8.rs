use crate::{
    Array, ArrayImpl, BUILTIN_EXPRESSION_NAMES, BinaryExpression, BinaryScalarFunction, BoolArray,
    BooleanOperator, CheckedBinaryExpression, CheckedBinaryScalarFunction,
    CheckedTernaryScalarFunction, CheckedUnaryScalarFunction, ColumnViewImpl, Expression, I32Array,
    PhysicalType, ScalarError, ScalarRefImpl, StringArray, TernaryExpression, UnaryExpression,
    build_boolean_expression, build_builtin_expression,
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
fn concatenates_borrowed_indexed_and_constant_strings() {
    let expression = build_builtin_expression("string_concat").unwrap();
    assert_eq!(
        expression.input_types(),
        &[PhysicalType::String, PhysicalType::String]
    );
    assert_eq!(expression.output_type(), PhysicalType::String);

    let values: ArrayImpl = StringArray::from_slice(&[Some("rust"), None, Some("data")]).into();
    let keys = [Some(2), Some(0), None, Some(1)];
    let inputs = [
        ColumnViewImpl::indexed(&keys, &values).unwrap(),
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
    let expression: Box<dyn Expression> =
        Box::new(BinaryExpression::new("string_length_add", StringLengthAdd));
    assert!(expression.evaluate(&[]).is_err());

    let inputs = [ColumnViewImpl::null(PhysicalType::String, 1)];
    assert!(expression.evaluate(&inputs).is_err());

    let inputs = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
        ColumnViewImpl::null(PhysicalType::String, 1),
    ];
    assert!(expression.evaluate(&inputs).is_err());
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

#[test]
fn boolean_expressions_delegate_through_the_erased_boundary() {
    // Day 7 deferred ledger: the three-valued Boolean expression must reach
    // the same rows and metadata through dyn Expression as through its
    // inherent evaluate.
    let and: Box<dyn Expression> = Box::new(build_boolean_expression(BooleanOperator::And));
    assert_eq!(and.name(), "boolean_and");
    assert_eq!(and.arity(), 2);
    assert_eq!(and.input_types(), &[PhysicalType::Bool, PhysicalType::Bool]);
    assert_eq!(and.output_type(), PhysicalType::Bool);

    let left: ArrayImpl = BoolArray::from_slice(&[Some(true), Some(false), None]).into();
    let right: ArrayImpl = BoolArray::from_slice(&[Some(false), Some(true), Some(false)]).into();
    let result = and
        .evaluate(&[ColumnViewImpl::array(&left), ColumnViewImpl::array(&right)])
        .unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&result)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(false), Some(false)]
    );

    let not: Box<dyn Expression> = Box::new(build_boolean_expression(BooleanOperator::Not));
    assert_eq!(not.name(), "boolean_not");
    assert_eq!(not.arity(), 1);
    assert_eq!(not.input_types(), &[PhysicalType::Bool]);
    let result = not.evaluate(&[ColumnViewImpl::array(&left)]).unwrap();
    assert_eq!(
        <&BoolArray>::try_from(&result)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(false), Some(true), None]
    );
}

struct CheckedNeg;

impl CheckedUnaryScalarFunction for CheckedNeg {
    type Input = i32;
    type Output = i32;

    fn evaluate<'a>(&self, input: i32) -> Result<i32, ScalarError> {
        Ok(input.wrapping_neg())
    }
}

struct CheckedAdd;

impl CheckedBinaryScalarFunction for CheckedAdd {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate<'a>(&self, left: i32, right: i32) -> Result<i32, ScalarError> {
        Ok(left.wrapping_add(right))
    }
}

struct CheckedClamp;

impl CheckedTernaryScalarFunction for CheckedClamp {
    type First = i32;
    type Second = i32;
    type Third = i32;
    type Output = i32;

    fn evaluate<'a>(&self, value: i32, lower: i32, upper: i32) -> Result<i32, ScalarError> {
        Ok(value.clamp(lower, upper))
    }
}

#[test]
fn erased_unary_shell_delegates_metadata_and_rows() {
    let expression: Box<dyn Expression> = Box::new(UnaryExpression::new("checked_neg", CheckedNeg));
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
fn erased_binary_shell_delegates_metadata_and_rows() {
    let expression: Box<dyn Expression> =
        Box::new(CheckedBinaryExpression::new("checked_add", CheckedAdd));
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
fn erased_ternary_shell_delegates_metadata_and_rows() {
    let expression: Box<dyn Expression> =
        Box::new(TernaryExpression::new("checked_clamp", CheckedClamp));
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
