use crate::{
    Array, ArrayImpl, CheckedTernaryScalarFunction, ColumnViewImpl, ExpressionError, I16Array,
    I64Array, PhysicalType, ScalarError, ScalarRefImpl, TernaryExpression, TypeMismatch,
    validate_expression_inputs,
};

// === Chapter 6 checkpoint 1 ===

#[test]
fn checkpoint_1_validates_arity_then_type_then_length_for_any_arity() {
    let wrong_arity = [ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 3)];
    assert_eq!(
        validate_expression_inputs(
            &wrong_arity,
            &[PhysicalType::Int32, PhysicalType::Int32],
        ),
        Err(ExpressionError::InputArityMismatch {
            expected: 2,
            actual: 1,
        })
    );

    let wrong_type_and_earlier_length = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 1),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
        ColumnViewImpl::constant(ScalarRefImpl::String("wrong"), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(6), 2),
    ];
    assert_eq!(
        validate_expression_inputs(
            &wrong_type_and_earlier_length,
            &[const { PhysicalType::Int32 }; 6],
        ),
        Err(ExpressionError::TypeMismatch(TypeMismatch {
            expected: PhysicalType::Int32,
            actual: PhysicalType::String,
        }))
    );

    let columns = [
        ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(2), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(3), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(4), 2),
        ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 1),
    ];
    assert_eq!(
        validate_expression_inputs(&columns[..4], &[const { PhysicalType::Int32 }; 4]),
        Ok(2)
    );
    assert_eq!(
        validate_expression_inputs(&columns, &[const { PhysicalType::Int32 }; 5]),
        Err(ExpressionError::InputLengthMismatch {
            expected: 2,
            actual: 1,
            input_index: 4,
        })
    );
}

// === Chapter 6 checkpoint 2 ===

struct MixedClamp;

impl CheckedTernaryScalarFunction for MixedClamp {
    type First = i16;
    type Second = i32;
    type Third = i64;
    type Output = i64;

    fn evaluate<'a>(
        &self,
        value: i16,
        lower: i32,
        upper: i64,
    ) -> Result<i64, ScalarError> {
        let value = i64::from(value);
        let lower = i64::from(lower);
        if lower > upper {
            return Err(ScalarError::InvalidClampBounds);
        }
        Ok(value.clamp(lower, upper))
    }
}

#[test]
fn checkpoint_2_runs_one_mixed_typed_ternary_loop() {
    let expression = TernaryExpression::new("mixed_clamp", MixedClamp);
    let values: ArrayImpl = I16Array::from_slice(&[Some(5), None, Some(25)]).into();
    let uppers: ArrayImpl = I64Array::from_values(vec![20, 0, 20]).into();
    let output = expression
        .evaluate(&[
            ColumnViewImpl::array(&values),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 3),
            ColumnViewImpl::array(&uppers),
        ])
        .unwrap();
    assert_eq!(
        <&I64Array>::try_from(&output)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        vec![Some(10), None, Some(20)]
    );

    let invalid_uppers: ArrayImpl = I64Array::from_values(vec![20, 0]).into();
    assert_eq!(
        expression.evaluate(&[
            ColumnViewImpl::constant(ScalarRefImpl::Int16(5), 2),
            ColumnViewImpl::constant(ScalarRefImpl::Int32(10), 2),
            ColumnViewImpl::array(&invalid_uppers),
        ]),
        Err(ExpressionError::ScalarEvaluation {
            function: "mixed_clamp",
            row: 1,
            error: ScalarError::InvalidClampBounds,
        })
    );
}
