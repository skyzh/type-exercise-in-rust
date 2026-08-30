use std::any::Any;
use std::future::Future;
use std::pin::Pin;

use crate::column::DenseI32Column;
use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, I32Array, Nullability,
    PhysicalType, Scalar, ScalarRefImpl, TypeMismatch,
};

/// A runtime-erased expression with discoverable physical metadata.
pub trait Expression: Any + Send + Sync {
    fn name(&self) -> &'static str;
    fn input_types(&self) -> &[PhysicalType];
    fn arity(&self) -> usize {
        self.input_types().len()
    }
    fn output_type(&self) -> PhysicalType;
    fn output_nullability(&self, inputs: &[Nullability]) -> Nullability {
        if inputs
            .iter()
            .all(|nullability| *nullability == Nullability::NonNull)
        {
            Nullability::NonNull
        } else {
            Nullability::Nullable
        }
    }
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>;
    fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)> {
        self.evaluate(inputs)
            .map(|output| (output, PrimitiveLoop::General))
    }
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

/// The loop selected for one binary evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveLoop {
    General,
    ArrayArray,
    ArrayConstant,
    ConstantArray,
    ConstantConstant,
}

/// One typed binary scalar function that can be lifted over nullable columns.
pub trait BinaryScalarFunction: Send + Sync + 'static {
    type Left: Scalar;
    type Right: Scalar;
    type Output: Scalar + Copy;

    fn evaluate<'a>(
        &self,
        left: <Self::Left as Scalar>::RefType<'a>,
        right: <Self::Right as Scalar>::RefType<'a>,
    ) -> Self::Output;
}

/// The Chapter 3 scalar function. Addition uses explicit wrapping semantics.
#[derive(Clone, Copy, Debug, Default)]
pub struct I32Add;

impl BinaryScalarFunction for I32Add {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate<'a>(&self, left: i32, right: i32) -> i32 {
        left.wrapping_add(right)
    }
}

/// Convert erased inputs once, then apply a typed scalar function row by row.
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
        anyhow::anyhow!(
            "input 0 type mismatch: expected {:?}, got {:?}",
            error.expected,
            error.actual
        )
    })?;
    let right = ColumnView::<F::Right>::try_from(right).map_err(|error| {
        anyhow::anyhow!(
            "input 1 type mismatch: expected {:?}, got {:?}",
            error.expected,
            error.actual
        )
    })?;
    evaluate_typed_binary(function, left, right)
}

fn evaluate_typed_binary<'a, F>(
    function: &F,
    left: ColumnView<'a, F::Left>,
    right: ColumnView<'a, F::Right>,
) -> anyhow::Result<ArrayImpl>
where
    F: BinaryScalarFunction,
{
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

/// An `i32` binary adapter with checked all-valid fast paths.
pub struct PrimitiveBinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}

impl<F> PrimitiveBinaryExpression<F>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32>,
{
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [PhysicalType::Int32, PhysicalType::Int32],
            function,
        }
    }

    pub fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)> {
        if inputs.len() != self.input_types.len() {
            anyhow::bail!(
                "input arity mismatch: expected {}, got {}",
                self.input_types.len(),
                inputs.len()
            );
        }
        for (input_index, (input, expected)) in inputs.iter().zip(&self.input_types).enumerate() {
            if input.physical_type() != *expected {
                anyhow::bail!(
                    "input {input_index} type mismatch: expected {expected:?}, got {:?}",
                    input.physical_type()
                );
            }
        }
        if inputs[0].len() != inputs[1].len() {
            anyhow::bail!(
                "input 1 length mismatch: expected {}, got {}",
                inputs[0].len(),
                inputs[1].len()
            );
        }

        let Some(left) = inputs[0].as_dense_i32() else {
            return Ok((
                evaluate_binary(&self.function, inputs[0].clone(), inputs[1].clone())?,
                PrimitiveLoop::General,
            ));
        };
        let Some(right) = inputs[1].as_dense_i32() else {
            return Ok((
                evaluate_binary(&self.function, inputs[0].clone(), inputs[1].clone())?,
                PrimitiveLoop::General,
            ));
        };
        debug_assert_eq!(left.len(), right.len());

        let (values, selected_loop) = match (left, right) {
            (DenseI32Column::Array(left), DenseI32Column::Array(right)) => (
                left.values()
                    .iter()
                    .copied()
                    .zip(right.values().iter().copied())
                    .map(|(left, right)| self.function.evaluate(left, right))
                    .collect(),
                PrimitiveLoop::ArrayArray,
            ),
            (DenseI32Column::Array(left), DenseI32Column::Constant { value: right, .. }) => (
                left.values()
                    .iter()
                    .copied()
                    .map(|left| self.function.evaluate(left, right))
                    .collect(),
                PrimitiveLoop::ArrayConstant,
            ),
            (DenseI32Column::Constant { value: left, .. }, DenseI32Column::Array(right)) => (
                right
                    .values()
                    .iter()
                    .copied()
                    .map(|right| self.function.evaluate(left, right))
                    .collect(),
                PrimitiveLoop::ConstantArray,
            ),
            (
                DenseI32Column::Constant { value: left, len },
                DenseI32Column::Constant { value: right, .. },
            ) => (
                (0..len)
                    .map(|_| self.function.evaluate(left, right))
                    .collect(),
                PrimitiveLoop::ConstantConstant,
            ),
        };

        Ok((I32Array::from_values(values).into(), selected_loop))
    }
}

impl<F> Expression for PrimitiveBinaryExpression<F>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32>,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn input_types(&self) -> &[PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> PhysicalType {
        PhysicalType::Int32
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate_with_loop(inputs).map(|(output, _)| output)
    }

    fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)> {
        PrimitiveBinaryExpression::evaluate_with_loop(self, inputs)
    }
}

/// Adapt one typed binary scalar function to the runtime expression interface.
pub type BinaryBatchKernel = for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
pub type BinaryLoopKernel =
    for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)>;

pub struct BinaryExpression {
    name: &'static str,
    input_types: [crate::PhysicalType; 2],
    output_type: crate::PhysicalType,
    kernel: BinaryBatchKernel,
    loop_kernel: Option<BinaryLoopKernel>,
}

impl BinaryExpression {
    pub fn new(
        name: &'static str,
        input_types: [crate::PhysicalType; 2],
        output_type: crate::PhysicalType,
        kernel: BinaryBatchKernel,
    ) -> Self {
        Self {
            name,
            input_types,
            output_type,
            kernel,
            loop_kernel: None,
        }
    }

    pub fn new_with_loop(
        name: &'static str,
        input_types: [crate::PhysicalType; 2],
        output_type: crate::PhysicalType,
        kernel: BinaryBatchKernel,
        loop_kernel: BinaryLoopKernel,
    ) -> Self {
        Self {
            name,
            input_types,
            output_type,
            kernel,
            loop_kernel: Some(loop_kernel),
        }
    }
}

impl Expression for BinaryExpression {
    fn name(&self) -> &'static str {
        self.name
    }

    fn input_types(&self) -> &[crate::PhysicalType] {
        &self.input_types
    }

    fn output_type(&self) -> crate::PhysicalType {
        self.output_type.clone()
    }

    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        self.evaluate_with_loop(inputs).map(|(output, _)| output)
    }

    fn evaluate_with_loop(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)> {
        if inputs.len() != self.arity() {
            anyhow::bail!(
                "input arity mismatch: expected {}, got {}",
                self.arity(),
                inputs.len()
            );
        }
        for (input_index, (input, expected)) in inputs.iter().zip(&self.input_types).enumerate() {
            if input.physical_type() != *expected {
                anyhow::bail!(
                    "input {input_index} type mismatch: expected {expected:?}, got {:?}",
                    input.physical_type()
                );
            }
        }
        if inputs[0].len() != inputs[1].len() {
            anyhow::bail!(
                "input 1 length mismatch: expected {}, got {}",
                inputs[0].len(),
                inputs[1].len()
            );
        }
        let (output, selected_loop) = match self.loop_kernel {
            Some(kernel) => kernel(inputs),
            None => (self.kernel)(inputs).map(|output| (output, PrimitiveLoop::General)),
        }?;
        if output.physical_type() != self.output_type {
            anyhow::bail!(
                "output type mismatch: expected {:?}, got {:?}",
                self.output_type,
                output.physical_type()
            );
        }
        Ok((output, selected_loop))
    }
}

fn evaluate_i32_add_batch(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<i32>::try_from(inputs[0].clone())?;
    let right = ColumnView::<i32>::try_from(inputs[1].clone())?;
    let mut output = <crate::I32Array as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        output.push(
            left.get(row)
                .zip(right.get(row))
                .map(|(left, right)| left.wrapping_add(right)),
        );
    }
    Ok(output.finish().into())
}

fn evaluate_i32_add_batch_with_loop(
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<(ArrayImpl, PrimitiveLoop)> {
    let Some(left) = inputs[0].as_dense_i32() else {
        return evaluate_i32_add_batch(inputs).map(|output| (output, PrimitiveLoop::General));
    };
    let Some(right) = inputs[1].as_dense_i32() else {
        return evaluate_i32_add_batch(inputs).map(|output| (output, PrimitiveLoop::General));
    };

    let (values, selected_loop) = match (left, right) {
        (DenseI32Column::Array(left), DenseI32Column::Array(right)) => (
            left.values()
                .iter()
                .copied()
                .zip(right.values().iter().copied())
                .map(|(left, right)| left.wrapping_add(right))
                .collect(),
            PrimitiveLoop::ArrayArray,
        ),
        (DenseI32Column::Array(left), DenseI32Column::Constant { value: right, .. }) => (
            left.values()
                .iter()
                .copied()
                .map(|left| left.wrapping_add(right))
                .collect(),
            PrimitiveLoop::ArrayConstant,
        ),
        (DenseI32Column::Constant { value: left, .. }, DenseI32Column::Array(right)) => (
            right
                .values()
                .iter()
                .copied()
                .map(|right| left.wrapping_add(right))
                .collect(),
            PrimitiveLoop::ConstantArray,
        ),
        (
            DenseI32Column::Constant { value: left, len },
            DenseI32Column::Constant { value: right, .. },
        ) => (
            (0..len).map(|_| left.wrapping_add(right)).collect(),
            PrimitiveLoop::ConstantConstant,
        ),
    };

    Ok((crate::I32Array::from_values(values).into(), selected_loop))
}

fn evaluate_string_concat_batch(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<String>::try_from(inputs[0].clone())?;
    let right = ColumnView::<String>::try_from(inputs[1].clone())?;
    let mut output = <crate::StringArray as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => output.try_push_with(|writer| {
                writer.push_str(left);
                writer.push_str(right);
                Ok::<_, std::convert::Infallible>(())
            })?,
            _ => output.push_null(),
        }
    }
    Ok(output.finish().into())
}

#[derive(Clone)]
struct BinaryBuiltin {
    name: &'static str,
    input_types: [crate::PhysicalType; 2],
    output_type: crate::PhysicalType,
    kernel: BinaryBatchKernel,
    loop_kernel: Option<BinaryLoopKernel>,
}

impl BinaryBuiltin {
    fn build(&self) -> BinaryExpression {
        let expression = BinaryExpression::new(
            self.name,
            self.input_types.clone(),
            self.output_type.clone(),
            self.kernel,
        );
        match self.loop_kernel {
            Some(loop_kernel) => BinaryExpression::new_with_loop(
                expression.name,
                expression.input_types,
                expression.output_type,
                expression.kernel,
                loop_kernel,
            ),
            None => expression,
        }
    }
}

macro_rules! define_builtin_expressions {
    ($( $name:literal => ($left:expr, $right:expr) -> $output:expr => $kernel:path $(, $loop_kernel:path)? ),+ $(,)?) => {
        pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &[$($name),+];

        const BUILTIN_EXPRESSIONS: &[BinaryBuiltin] = &[
            $(BinaryBuiltin {
                name: $name,
                input_types: [$left, $right],
                output_type: $output,
                kernel: $kernel,
                loop_kernel: define_builtin_expressions!(@loop $($loop_kernel)?),
            }),+
        ];

        pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
            BUILTIN_EXPRESSIONS
                .iter()
                .find(|builtin| builtin.name == name)
                .map(|builtin| Box::new(builtin.build()) as Box<dyn Expression>)
        }
    };
    (@loop) => { None };
    (@loop $loop_kernel:path) => { Some($loop_kernel) };
}

define_builtin_expressions! {
    "i32_add" => (crate::PhysicalType::Int32, crate::PhysicalType::Int32)
        -> crate::PhysicalType::Int32 => evaluate_i32_add_batch, evaluate_i32_add_batch_with_loop,
    "string_concat" => (crate::PhysicalType::String, crate::PhysicalType::String)
        -> crate::PhysicalType::String => evaluate_string_concat_batch,
}
