//! Physical expression shell, validation, and vectorization templates.

mod binary;
mod fallback;
mod primitive_i32;
mod ternary;
mod unary;
mod writer;

use std::any::Any;
use std::future::Future;
use std::pin::Pin;

use crate::{ArrayImpl, ColumnViewImpl, PhysicalType};

pub use binary::{
    auto_vectorize_binary, evaluate_binary, evaluate_nullable_binary, try_evaluate_binary,
};
pub use primitive_i32::auto_vectorize_primitive_i32;
pub use ternary::{
    auto_vectorize_ternary, evaluate_ternary, try_auto_vectorize_ternary, try_evaluate_ternary,
};
pub use unary::{auto_vectorize_unary, evaluate_nullable_unary, evaluate_unary};
pub use writer::evaluate_writer_binary;

/// A runtime-erased expression with discoverable physical metadata.
pub trait Expression: Any + Send + Sync {
    fn name(&self) -> &'static str;
    fn input_types(&self) -> &[PhysicalType];
    fn arity(&self) -> usize {
        self.input_types().len()
    }
    fn output_type(&self) -> PhysicalType;
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>;
}

/// One erased future for one complete batch evaluation.
pub type BatchFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a>>;

/// Evaluate one borrowed batch while keeping the future type compiler-known.
#[allow(clippy::manual_async_fn)]
pub fn evaluate_static<'a, E>(
    expression: &'a E,
    inputs: &'a [ColumnViewImpl<'a>],
) -> impl Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a
where
    E: Expression + ?Sized,
{
    async move { expression.evaluate(inputs) }
}

/// The anonymous future returned here is not part of an `Unpin` contract.
/// Pin it before polling instead of assuming it can be moved after polling starts.
///
/// ```compile_fail
/// use type_exercise_core::{
///     ArrayImpl, BatchExpression, ColumnViewImpl, PhysicalType, evaluate_static,
/// };
///
/// fn kernel(_inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
///     unreachable!()
/// }
/// let expression = BatchExpression::new(
///     "add",
///     [PhysicalType::Int32, PhysicalType::Int32],
///     PhysicalType::Int32,
///     kernel,
/// );
/// let inputs: [ColumnViewImpl<'_>; 0] = [];
/// let mut future = evaluate_static(&expression, &inputs);
/// let _pinned = std::pin::Pin::new(&mut future);
/// ```
const _: () = ();

/// A dyn-compatible asynchronous boundary around one synchronous batch evaluation.
pub trait AsyncExpression: Send + Sync {
    fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
}

/// Adapt an existing erased physical expression without changing its evaluation semantics.
pub struct AsyncExpressionAdapter {
    expression: Box<dyn Expression>,
}

impl AsyncExpressionAdapter {
    pub fn new(expression: Box<dyn Expression>) -> Self {
        Self { expression }
    }
}

impl AsyncExpression for AsyncExpressionAdapter {
    fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a> {
        Box::pin(async move { self.expression.evaluate(inputs) })
    }
}

/// Validate arity, physical types, and row counts before evaluating a batch.
pub fn validate_expression_inputs(
    inputs: &[ColumnViewImpl<'_>],
    expected_types: &[PhysicalType],
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
pub type BatchKernel = for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;

/// The sole fixed-arity physical expression shell.
pub struct BatchExpression<const N: usize> {
    name: &'static str,
    input_types: [PhysicalType; N],
    output_type: PhysicalType,
    kernel: BatchKernel,
}

impl<const N: usize> BatchExpression<N> {
    pub fn new(
        name: &'static str,
        input_types: [PhysicalType; N],
        output_type: PhysicalType,
        kernel: BatchKernel,
    ) -> Self {
        Self {
            name,
            input_types,
            output_type,
            kernel,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn input_types(&self) -> &[PhysicalType; N] {
        &self.input_types
    }

    pub fn output_type(&self) -> PhysicalType {
        self.output_type.clone()
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        let input_len = validate_expression_inputs(inputs, &self.input_types)?;
        let output = (self.kernel)(inputs)?;
        if output.physical_type() != self.output_type {
            anyhow::bail!(
                "output type mismatch: expected {:?}, got {:?}",
                self.output_type,
                output.physical_type()
            );
        }
        if output.len() != input_len {
            anyhow::bail!(
                "output length mismatch: expected {input_len}, got {}",
                output.len()
            );
        }
        Ok(output)
    }
}

impl<const N: usize> Expression for BatchExpression<N> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> PhysicalType {
        self.output_type.clone()
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate(inputs)
    }
}
