use crate::{
    Array, ArrayImpl, ColumnViewImpl, DataType, Expression, ExpressionError, FunctionRegistry,
    I32Add, I32Array, Nullability, PhysicalType, PrimitiveBinaryExpression, PrimitiveLoop,
    ScalarRefImpl, StringArray, TypeMismatch, build_builtin_expression,
};

fn i32_values(array: &ArrayImpl) -> Vec<Option<i32>> {
    <&I32Array>::try_from(array).unwrap().iter().collect()
}

#[test]
fn proves_physical_nullability_at_the_column_boundary() {
    let nullable: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
    assert_eq!(
        ColumnViewImpl::array(&nullable).nullability(),
        Nullability::Nullable
    );
    assert!(ColumnViewImpl::try_non_null_array(&nullable).is_none());

    let dense: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    assert_eq!(
        ColumnViewImpl::array(&dense).nullability(),
        Nullability::Nullable
    );
    assert_eq!(
        ColumnViewImpl::try_non_null_array(&dense)
            .unwrap()
            .nullability(),
        Nullability::NonNull
    );

    let built_dense: ArrayImpl = I32Array::from_slice(&[Some(4), Some(5)]).into();
    assert!(ColumnViewImpl::try_non_null_array(&built_dense).is_some());
    let empty: ArrayImpl = I32Array::from_values(Vec::new()).into();
    assert!(ColumnViewImpl::try_non_null_array(&empty).is_some());

    assert_eq!(
        ColumnViewImpl::constant(ScalarRefImpl::Int32(7), 2).nullability(),
        Nullability::NonNull
    );
    assert_eq!(
        ColumnViewImpl::null(PhysicalType::Int32, 2).nullability(),
        Nullability::Nullable
    );
    let indices = [Some(0)];
    assert_eq!(
        ColumnViewImpl::indexed(&indices, &dense)
            .unwrap()
            .nullability(),
        Nullability::Nullable
    );
}

#[test]
fn selects_the_dense_array_array_loop() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let left: ArrayImpl = I32Array::from_values(vec![i32::MAX, 20, 30]).into();
    let right: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::try_non_null_array(&left).unwrap(),
            ColumnViewImpl::try_non_null_array(&right).unwrap(),
        ])
        .unwrap();

    assert_eq!(selected, PrimitiveLoop::ArrayArray);
    assert_eq!(
        i32_values(&output),
        vec![Some(i32::MIN), Some(22), Some(33)]
    );
}

#[test]
fn selects_all_three_dense_constant_loops() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let values: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let cases = [
        (
            [
                ColumnViewImpl::try_non_null_array(&values).unwrap(),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
            ],
            PrimitiveLoop::ArrayConstant,
            vec![Some(11), Some(12), Some(13)],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
                ColumnViewImpl::try_non_null_array(&values).unwrap(),
            ],
            PrimitiveLoop::ConstantArray,
            vec![Some(11), Some(12), Some(13)],
        ),
        (
            [
                ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
                ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 3),
            ],
            PrimitiveLoop::ConstantConstant,
            vec![Some(12), Some(12), Some(12)],
        ),
    ];

    for (inputs, expected_loop, expected_values) in cases {
        let (output, selected) = expression.evaluate_with_loop(&inputs).unwrap();
        assert_eq!(selected, expected_loop);
        assert_eq!(i32_values(&output), expected_values);
    }
}

#[test]
fn delegates_nullable_arrays_and_null_constants_to_the_general_loop() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let nullable: ArrayImpl = I32Array::from_slice(&[Some(1), None, Some(3)]).into();
    let dense: ArrayImpl = I32Array::from_values(vec![10, 20, 30]).into();

    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::array(&nullable),
            ColumnViewImpl::array(&dense),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::General);
    assert_eq!(i32_values(&output), vec![Some(11), None, Some(33)]);

    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::array(&dense),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 3),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::General);
    assert_eq!(i32_values(&output), vec![Some(11), Some(21), Some(31)]);

    let (output, selected) = expression
        .evaluate_with_loop(&[
            ColumnViewImpl::array(&dense),
            ColumnViewImpl::null(PhysicalType::Int32, 3),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::General);
    assert_eq!(i32_values(&output), vec![None, None, None]);
}

#[test]
fn delegates_dictionaries_to_the_general_loop() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let dictionary_values: ArrayImpl = I32Array::from_values(vec![4, 8]).into();
    let keys = [Some(1), None, Some(0)];
    let right: ArrayImpl = I32Array::from_values(vec![1, 2, 3]).into();
    let dictionary = ColumnViewImpl::indexed(&keys, &dictionary_values).unwrap();
    let inputs = [
        dictionary,
        ColumnViewImpl::try_non_null_array(&right).unwrap(),
    ];

    let (output, selected) = expression.evaluate_with_loop(&inputs).unwrap();
    assert_eq!(selected, PrimitiveLoop::General);
    assert_eq!(i32_values(&output), vec![Some(9), None, Some(7)]);
}

#[test]
fn preserves_runtime_type_arity_and_length_errors() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    let integers: ArrayImpl = I32Array::from_values(vec![1, 2]).into();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();

    assert_eq!(
        expression.evaluate(&[ColumnViewImpl::array(&integers)]),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        expression.evaluate(&[
            ColumnViewImpl::array(&strings),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ]),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }))
    );
    assert_eq!(
        expression.evaluate(&[
            ColumnViewImpl::array(&integers),
            ColumnViewImpl::array(&strings),
        ]),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }))
    );
    assert_eq!(
        expression.evaluate(&[
            ColumnViewImpl::try_non_null_array(&integers).unwrap(),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1),
        ]),
        Err(ExpressionError::InputLengthMismatch {
            expected: 2,
            actual: 1,
            input_index: 1,
        })
    );
}

#[test]
fn propagates_nullability_through_physical_and_bound_expressions() {
    let expression = PrimitiveBinaryExpression::new("i32_add", I32Add);
    assert_eq!(
        expression.output_nullability(&[Nullability::NonNull, Nullability::NonNull]),
        Nullability::NonNull
    );
    assert_eq!(
        expression.output_nullability(&[Nullability::NonNull, Nullability::Nullable]),
        Nullability::Nullable
    );

    let bound = FunctionRegistry::with_builtins()
        .bind_binary("+", DataType::Integer, DataType::Integer)
        .unwrap();
    assert_eq!(
        bound.output_nullability(&[Nullability::NonNull, Nullability::NonNull]),
        Nullability::NonNull
    );
    assert_eq!(
        bound.output_nullability(&[Nullability::Nullable, Nullability::NonNull]),
        Nullability::Nullable
    );
}

#[test]
fn keeps_binding_and_non_primitive_catalog_entries_working() {
    let add = FunctionRegistry::with_builtins()
        .bind_binary("+", DataType::Integer, DataType::Integer)
        .unwrap();
    let integers: ArrayImpl = I32Array::from_values(vec![1, 2]).into();
    let (output, selected) = add
        .evaluate_with_loop(&[
            ColumnViewImpl::try_non_null_array(&integers).unwrap(),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 2),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::ArrayConstant);
    assert_eq!(i32_values(&output), vec![Some(6), Some(7)]);

    let concat = build_builtin_expression("string_concat").unwrap();
    let strings: ArrayImpl = StringArray::from_slice(&[Some("data"), None]).into();
    let (output, selected) = concat
        .evaluate_with_loop(&[
            ColumnViewImpl::array(&strings),
            ColumnViewImpl::constant(ScalarRefImpl::String("base"), 2),
        ])
        .unwrap();
    assert_eq!(selected, PrimitiveLoop::General);
    let output = <&StringArray>::try_from(&output).unwrap();
    assert_eq!(
        output.iter().collect::<Vec<_>>(),
        vec![Some("database"), None]
    );
}
