use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, Scalar, ScalarRefImpl, TypeMismatch,
};

pub trait BinaryScalarFunction {
    type Left: Scalar;
    type Right: Scalar;
    type Output: Scalar + Copy;

    fn evaluate<'a>(
        &self,
        left: <Self::Left as Scalar>::RefType<'a>,
        right: <Self::Right as Scalar>::RefType<'a>,
    ) -> Self::Output;
}

pub fn evaluate_binary<'a, F>(
    function: &F,
    left: ColumnViewImpl<'a>,
    right: ColumnViewImpl<'a>,
) -> anyhow::Result<ArrayImpl>
where
    F: BinaryScalarFunction,
    <F::Left as Scalar>::ArrayType: 'a,
    <F::Right as Scalar>::ArrayType: 'a,
    &'a <F::Left as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    &'a <F::Right as Scalar>::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    <F::Left as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    <F::Right as Scalar>::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    let left = ColumnView::<F::Left>::try_from(left).map_err(|error| {
        let context = format!(
            "input 0 type mismatch: expected {:?}, got {:?}",
            error.expected, error.actual
        );
        anyhow::Error::new(error).context(context)
    })?;
    let right = ColumnView::<F::Right>::try_from(right).map_err(|error| {
        let context = format!(
            "input 1 type mismatch: expected {:?}, got {:?}",
            error.expected, error.actual
        );
        anyhow::Error::new(error).context(context)
    })?;
    if left.len() != right.len() {
        anyhow::bail!(
            "input 1 length mismatch: expected {}, got {}",
            left.len(),
            right.len()
        );
    }

    let mut output =
        <<F::Output as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => Some(function.evaluate(left, right)),
            _ => None,
        };
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

use crate::PhysicalType as BatchPhysicalType;

fn validate_expression_inputs(
    inputs: &[ColumnViewImpl<'_>],
    expected_types: &[BatchPhysicalType],
) -> anyhow::Result<usize> {
    if inputs.len() != expected_types.len() {
        anyhow::bail!(
            "input arity mismatch: expected {}, got {}",
            expected_types.len(),
            inputs.len()
        );
    }
    for (input_index, (input, expected)) in inputs.iter().zip(expected_types).enumerate() {
        if input.physical_type() != *expected {
            anyhow::bail!(
                "input {input_index} type mismatch: expected {expected:?}, got {:?}",
                input.physical_type()
            );
        }
    }
    let len = inputs.first().map_or(0, ColumnViewImpl::len);
    for (input_index, input) in inputs.iter().enumerate().skip(1) {
        if input.len() != len {
            anyhow::bail!(
                "input {input_index} length mismatch: expected {len}, got {}",
                input.len()
            );
        }
    }
    Ok(len)
}

/// One monomorphized evaluator for a complete input batch.
pub type BatchKernel<const N: usize> =
    for<'a> fn(&BatchExpression<N>, &[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;

/// A fixed-arity expression whose only callable operation is vectorized.
#[allow(dead_code)]
pub struct BatchExpression<const N: usize> {
    name: &'static str,
    input_types: [BatchPhysicalType; N],
    output_type: BatchPhysicalType,
    kernel: BatchKernel<N>,
}

impl<const N: usize> BatchExpression<N> {
    pub fn new(
        name: &'static str,
        input_types: [BatchPhysicalType; N],
        output_type: BatchPhysicalType,
        kernel: BatchKernel<N>,
    ) -> Self {
        Self {
            name,
            input_types,
            output_type,
            kernel,
        }
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        validate_expression_inputs(inputs, &self.input_types)?;
        (self.kernel)(self, inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PhysicalType, StringArray};

    struct I32Add;

    impl BinaryScalarFunction for I32Add {
        type Left = i32;
        type Right = i32;
        type Output = i32;

        fn evaluate<'a>(&self, left: i32, right: i32) -> i32 {
            left.wrapping_add(right)
        }
    }

    #[test]
    fn conversion_errors_preserve_their_typed_causes() {
        let strings: ArrayImpl = StringArray::from_slice(&[Some("wrong")]).into();
        let valid = ColumnViewImpl::constant(ScalarRefImpl::Int32(1), 1);

        let left_error = evaluate_binary(
            &I32Add,
            ColumnViewImpl::array(&strings),
            valid.clone(),
        )
        .unwrap_err();
        assert_eq!(
            left_error.to_string(),
            "input 0 type mismatch: expected Int32, got String"
        );
        let left_cause = left_error
            .downcast_ref::<TypeMismatch>()
            .expect("left conversion must preserve its typed cause");
        assert_eq!(left_cause.expected, PhysicalType::Int32);
        assert_eq!(left_cause.actual, PhysicalType::String);
        assert_eq!(left_error.chain().count(), 2);

        let right_error = evaluate_binary(&I32Add, valid, ColumnViewImpl::array(&strings))
            .unwrap_err();
        assert_eq!(
            right_error.to_string(),
            "input 1 type mismatch: expected Int32, got String"
        );
        let right_cause = right_error
            .downcast_ref::<TypeMismatch>()
            .expect("right conversion must preserve its typed cause");
        assert_eq!(right_cause.expected, PhysicalType::Int32);
        assert_eq!(right_cause.actual, PhysicalType::String);
        assert_eq!(right_error.chain().count(), 2);
    }
}
