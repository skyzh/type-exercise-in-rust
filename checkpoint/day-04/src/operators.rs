use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, ExpressionError, PhysicalType,
    Scalar, ScalarError, ScalarRefImpl, TypeMismatch,
};

pub trait CheckedUnaryScalarFunction {
    type Input: Scalar;
    type Output: Scalar;
    fn evaluate<'a>(
        &self,
        input: <Self::Input as Scalar>::RefType<'a>,
    ) -> Result<Self::Output, ScalarError>;
}

pub trait CheckedBinaryScalarFunction {
    type Left: Scalar;
    type Right: Scalar;
    type Output: Scalar;
    fn evaluate<'a>(
        &self,
        left: <Self::Left as Scalar>::RefType<'a>,
        right: <Self::Right as Scalar>::RefType<'a>,
    ) -> Result<Self::Output, ScalarError>;
}

pub struct UnaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 1],
    function: F,
}

impl<F: CheckedUnaryScalarFunction> UnaryExpression<F> {
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [F::Input::PHYSICAL_TYPE],
            function,
        }
    }
}

pub struct CheckedBinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}

impl<F: CheckedBinaryScalarFunction> CheckedBinaryExpression<F> {
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [F::Left::PHYSICAL_TYPE, F::Right::PHYSICAL_TYPE],
            function,
        }
    }
}

fn validate_expression_inputs(
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

impl<F> UnaryExpression<F>
where
    F: CheckedUnaryScalarFunction,
    <F::Input as Scalar>::ArrayType: 'static,
    for<'a> &'a <F::Input as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> <F::Input as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_expression_inputs(inputs, &self.input_types)?;
        let input = ColumnView::<F::Input>::try_from(inputs[0].clone())?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match input.get(row) {
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

impl<F> CheckedBinaryExpression<F>
where
    F: CheckedBinaryScalarFunction,
    <F::Left as Scalar>::ArrayType: 'static,
    <F::Right as Scalar>::ArrayType: 'static,
    for<'a> &'a <F::Left as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a <F::Right as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> <F::Left as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> <F::Right as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl, ExpressionError> {
        let len = validate_expression_inputs(inputs, &self.input_types)?;
        let left = ColumnView::<F::Left>::try_from(inputs[0].clone())?;
        let right = ColumnView::<F::Right>::try_from(inputs[1].clone())?;
        let mut output = <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(len);
        for row in 0..len {
            let value = match (left.get(row), right.get(row)) {
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
