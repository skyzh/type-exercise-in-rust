use std::any::Any;

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

/// Lift a preselected borrowed scalar function over two strict nullable columns.
pub fn evaluate_borrowed_binary<'a, L, R, O, F>(
    left: ColumnViewImpl<'a>,
    right: ColumnViewImpl<'a>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar,
    R: Scalar,
    O: Scalar + Copy,
    F: Fn(L::RefType<'a>, R::RefType<'a>) -> O,
    L::ArrayType: 'a,
    R::ArrayType: 'a,
    &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[L::PHYSICAL_TYPE, R::PHYSICAL_TYPE],
    )?;
    let left = ColumnView::<L>::try_from(left)?;
    let right = ColumnView::<R>::try_from(right)?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = left
            .get(row)
            .zip(right.get(row))
            .map(|(left, right)| function(left, right));
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
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

/// Lift one infallible scalar function over two nullable columns.
pub fn auto_vectorize_binary<L, R, O, F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar + Copy,
    R: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(L, R) -> O,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[L::PHYSICAL_TYPE, R::PHYSICAL_TYPE],
    )?;
    let left = ColumnView::<L>::try_from(left)?;
    let right = ColumnView::<R>::try_from(right)?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = left
            .get(row)
            .zip(right.get(row))
            .map(|(left, right)| function(left, right));
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

/// Lift one infallible scalar function over a nullable unary column.
pub fn evaluate_unary<I, O, F>(input: ColumnViewImpl<'_>, function: F) -> anyhow::Result<ArrayImpl>
where
    I: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(I) -> O,
    for<'a> I: Scalar<RefType<'a> = I>,
    for<'a> &'a I::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> I::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    evaluate_nullable_unary::<I, O, _>(input, |value| Ok(value.map(&function)))
}

/// Lift one nullable-aware scalar function over a unary column.
pub fn evaluate_nullable_unary<I, O, F>(
    input: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    I: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(Option<I>) -> anyhow::Result<Option<O>>,
    for<'a> I: Scalar<RefType<'a> = I>,
    for<'a> &'a I::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> I::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(std::slice::from_ref(&input), &[I::PHYSICAL_TYPE])?;
    let input = ColumnView::<I>::try_from(input)?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(input.len());
    for row in 0..input.len() {
        let value = function(input.get(row))?;
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

/// Lift one fallible scalar function over two nullable columns.
pub fn try_evaluate_binary<L, R, O, F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function_name: &str,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar + Copy,
    R: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(L, R) -> anyhow::Result<O>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    let mut row = 0;
    evaluate_nullable_binary::<L, R, O, _>(left, right, |left, right| {
        let result = left
            .zip(right)
            .map(|(left, right)| function(left, right))
            .transpose()
            .map_err(|error| {
                if function_name.is_empty() {
                    anyhow::anyhow!("row {row}: {error}")
                } else {
                    anyhow::anyhow!("function `{function_name}` failed at row {row}: {error}")
                }
            });
        row += 1;
        result
    })
}

/// Lift one nullable-aware scalar function over two columns.
pub fn evaluate_nullable_binary<L, R, O, F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    mut function: F,
) -> anyhow::Result<ArrayImpl>
where
    L: Scalar + Copy,
    R: Scalar + Copy,
    O: Scalar + Copy,
    F: FnMut(Option<L>, Option<R>) -> anyhow::Result<Option<O>>,
    for<'a> L: Scalar<RefType<'a> = L>,
    for<'a> R: Scalar<RefType<'a> = R>,
    for<'a> &'a L::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a R::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> L::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> R::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[L::PHYSICAL_TYPE, R::PHYSICAL_TYPE],
    )?;
    let left = ColumnView::<L>::try_from(left)?;
    let right = ColumnView::<R>::try_from(right)?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        let value = function(left.get(row), right.get(row))?;
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

/// Lift one fallible scalar function over three nullable columns.
pub fn try_evaluate_ternary<A, B, C, O, F>(
    first: ColumnViewImpl<'_>,
    second: ColumnViewImpl<'_>,
    third: ColumnViewImpl<'_>,
    function_name: &str,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    A: Scalar + Copy,
    B: Scalar + Copy,
    C: Scalar + Copy,
    O: Scalar + Copy,
    F: Fn(A, B, C) -> anyhow::Result<O>,
    for<'a> A: Scalar<RefType<'a> = A>,
    for<'a> B: Scalar<RefType<'a> = B>,
    for<'a> C: Scalar<RefType<'a> = C>,
    for<'a> &'a A::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a B::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> &'a C::ArrayType: TryFrom<&'a ArrayImpl, Error = TypeMismatch>,
    for<'a> A::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> B::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
    for<'a> C::RefType<'a>: TryFrom<ScalarRefImpl<'a>, Error = TypeMismatch>,
{
    validate_expression_inputs(
        &[first.clone(), second.clone(), third.clone()],
        &[A::PHYSICAL_TYPE, B::PHYSICAL_TYPE, C::PHYSICAL_TYPE],
    )?;
    let first = ColumnView::<A>::try_from(first)?;
    let second = ColumnView::<B>::try_from(second)?;
    let third = ColumnView::<C>::try_from(third)?;
    let mut output = <<O as Scalar>::ArrayType as Array>::Builder::with_capacity(first.len());
    for row in 0..first.len() {
        let value = match (first.get(row), second.get(row), third.get(row)) {
            (Some(first), Some(second), Some(third)) => {
                Some(function(first, second, third).map_err(|error| {
                    anyhow::anyhow!("function `{function_name}` failed at row {row}: {error}")
                })?)
            }
            _ => None,
        };
        output.push(value.as_ref().map(Scalar::as_scalar_ref));
    }
    Ok(output.finish().into())
}

/// Lift one strict string scalar function into the transactional writer loop.
pub fn evaluate_writer_binary<F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: for<'a> Fn(&str, &str, crate::Writer<'a>) -> crate::WriterUsed<'a>,
{
    validate_expression_inputs(
        &[left.clone(), right.clone()],
        &[PhysicalType::String, PhysicalType::String],
    )?;
    let left = ColumnView::<String>::try_from(left)?;
    let right = ColumnView::<String>::try_from(right)?;
    let mut output = <crate::StringArray as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => {
                function(left, right, output.writer()).into_builder();
            }
            _ => output.push_null(),
        }
    }
    Ok(output.finish().into())
}

/// An `i32` binary adapter with checked all-valid fast paths.
pub struct PrimitiveBinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}

fn dense_array_array<F>(function: &F, left: &I32Array, right: &I32Array) -> Vec<i32>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32>,
{
    let mut output = Vec::with_capacity(left.values().len());
    for row in 0..left.values().len() {
        let left = left.values()[row];
        let right = right.values()[row];
        output.push(function.evaluate(left, right));
    }
    output
}

fn dense_array_constant<F>(function: &F, left: &I32Array, right: i32) -> Vec<i32>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32>,
{
    let mut output = Vec::with_capacity(left.values().len());
    for row in 0..left.values().len() {
        let left = left.values()[row];
        output.push(function.evaluate(left, right));
    }
    output
}

fn dense_constant_array<F>(function: &F, left: i32, right: &I32Array) -> Vec<i32>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32>,
{
    let mut output = Vec::with_capacity(right.values().len());
    for row in 0..right.values().len() {
        let right = right.values()[row];
        output.push(function.evaluate(left, right));
    }
    output
}

fn dense_constant_constant<F>(function: &F, left: i32, right: i32, len: usize) -> Vec<i32>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32>,
{
    let mut output = Vec::with_capacity(len);
    for _row in 0..len {
        output.push(function.evaluate(left, right));
    }
    output
}

impl<F> PrimitiveBinaryExpression<F>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32> + Send + Sync + 'static,
{
    pub fn new(name: &'static str, function: F) -> Self {
        Self {
            name,
            input_types: [PhysicalType::Int32, PhysicalType::Int32],
            function,
        }
    }

    pub fn output_nullability(&self, inputs: &[Nullability]) -> Nullability {
        <Self as Expression>::output_nullability(self, inputs)
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        <Self as Expression>::evaluate(self, inputs)
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
                dense_array_array(&self.function, left, right),
                PrimitiveLoop::ArrayArray,
            ),
            (DenseI32Column::Array(left), DenseI32Column::Constant { value: right, .. }) => (
                dense_array_constant(&self.function, left, right),
                PrimitiveLoop::ArrayConstant,
            ),
            (DenseI32Column::Constant { value: left, .. }, DenseI32Column::Array(right)) => (
                dense_constant_array(&self.function, left, right),
                PrimitiveLoop::ConstantArray,
            ),
            (
                DenseI32Column::Constant { value: left, len },
                DenseI32Column::Constant { value: right, .. },
            ) => (
                dense_constant_constant(&self.function, left, right, len),
                PrimitiveLoop::ConstantConstant,
            ),
        };

        Ok((I32Array::from_values(values).into(), selected_loop))
    }
}

impl<F> Expression for PrimitiveBinaryExpression<F>
where
    F: BinaryScalarFunction<Left = i32, Right = i32, Output = i32> + Send + Sync + 'static,
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
    reports_scalar_rows: bool,
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
            reports_scalar_rows: false,
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
            reports_scalar_rows: false,
        }
    }

    #[doc(hidden)]
    pub fn new_with_scalar_rows(
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
            reports_scalar_rows: true,
        }
    }

    pub fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        <Self as Expression>::evaluate(self, inputs)
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
        }
        .map_err(|error| {
            if self.reports_scalar_rows {
                anyhow::anyhow!("function `{}` failed at {error}", self.name)
            } else {
                error
            }
        })?;
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
