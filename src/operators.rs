use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnViewImpl, ExpressionError, PhysicalType, Scalar,
    ScalarError, ScalarRefImpl, TypeMismatch,
};

pub trait CheckedUnaryScalarFunction {
    type Output: Scalar;
    fn evaluate(&self, input: ScalarRefImpl<'_>) -> Result<Self::Output, ScalarError>;
}

pub trait CheckedBinaryScalarFunction {
    type Output: Scalar;
    fn evaluate(
        &self,
        left: ScalarRefImpl<'_>,
        right: ScalarRefImpl<'_>,
    ) -> Result<Self::Output, ScalarError>;
}

pub struct UnaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 1],
    function: F,
}

impl<F> UnaryExpression<F> {
    pub fn new(name: &'static str, input_types: [PhysicalType; 1], function: F) -> Self {
        Self {
            name,
            input_types,
            function,
        }
    }
}

pub struct CheckedBinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}

impl<F> CheckedBinaryExpression<F> {
    pub fn new(name: &'static str, input_types: [PhysicalType; 2], function: F) -> Self {
        Self {
            name,
            input_types,
            function,
        }
    }
}

fn validate_inputs(
    inputs: &[ColumnViewImpl<'_>],
    expected_types: &[PhysicalType],
) -> Result<usize, ExpressionError> {
    if inputs.len() != expected_types.len() {
        return Err(ExpressionError::InputArityMismatch {
            expected: expected_types.len(),
            actual: inputs.len(),
        });
    }
    for (input, expected) in inputs.iter().zip(expected_types) {
        if input.physical_type() != *expected {
            return Err(TypeMismatch {
                expected: expected.clone(),
                actual: input.physical_type(),
            }
            .into());
        }
    }
    let len = inputs.first().map_or(0, ColumnViewImpl::len);
    for (input_index, input) in inputs.iter().enumerate().skip(1) {
        if input.len() != len {
            return Err(ExpressionError::InputLengthMismatch {
                expected: len,
                actual: input.len(),
                input_index,
            });
        }
    }
    Ok(len)
}

impl<F: CheckedUnaryScalarFunction> UnaryExpression<F> {
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_inputs(inputs, &self.input_types)?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match inputs[0].get(row) {
                Some(input) => Some(self.function.evaluate(input).map_err(|error| {
                    ExpressionError::ScalarEvaluation {
                        function: self.name,
                        row,
                        error,
                    }
                })?),
                None => None,
            };
            output.push(value.as_ref().map(Scalar::as_scalar_ref));
        }
        Ok(output.finish().into())
    }
}

impl<F: CheckedBinaryScalarFunction> CheckedBinaryExpression<F> {
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_inputs(inputs, &self.input_types)?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match (inputs[0].get(row), inputs[1].get(row)) {
                (Some(left), Some(right)) => {
                    Some(self.function.evaluate(left, right).map_err(|error| {
                        ExpressionError::ScalarEvaluation {
                            function: self.name,
                            row,
                            error,
                        }
                    })?)
                }
                _ => None,
            };
            output.push(value.as_ref().map(Scalar::as_scalar_ref));
        }
        Ok(output.finish().into())
    }
}
