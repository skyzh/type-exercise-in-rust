use std::any::Any;

use crate::{
    Array, ArrayBuilder, ArrayImpl, ColumnView, ColumnViewImpl, Scalar, ScalarRefImpl, TypeMismatch,
};

pub trait Expression: Any + Send + Sync {
    fn name(&self) -> &'static str;
    fn input_types(&self) -> &[crate::PhysicalType];
    fn arity(&self) -> usize {
        self.input_types().len()
    }
    fn output_type(&self) -> crate::PhysicalType;
    fn evaluate(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>;
}
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

#[derive(Clone, Copy, Debug, Default)]
pub struct I32Add;

impl BinaryScalarFunction for I32Add {
    type Left = i32;
    type Right = i32;
    type Output = i32;

    fn evaluate(&self, left: i32, right: i32) -> i32 {
        left.wrapping_add(right)
    }
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

pub type BinaryBatchKernel = for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;

pub struct BinaryExpression {
    name: &'static str,
    input_types: [crate::PhysicalType; 2],
    output_type: crate::PhysicalType,
    kernel: BinaryBatchKernel,
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
            reports_scalar_rows: false,
        }
    }

    pub(crate) fn new_with_scalar_rows(
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
        let output = (self.kernel)(inputs).map_err(|error| {
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
        Ok(output)
    }
}

fn evaluate_i32_add_batch(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    crate::operators::auto_vectorize_binary::<i32, i32, i32, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        i32::wrapping_add,
    )
}

fn concat_strings<'a>(left: &str, right: &str, writer: crate::Writer<'a>) -> crate::WriterUsed<'a> {
    writer.write(|value| {
        value.push_str(left);
        value.push_str(right);
    })
}

fn evaluate_string_concat_batch(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    let left = ColumnView::<String>::try_from(inputs[0].clone())?;
    let right = ColumnView::<String>::try_from(inputs[1].clone())?;
    let mut output = <crate::StringArray as Array>::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => {
                concat_strings(left, right, output.writer()).into_builder();
            }
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
}

impl BinaryBuiltin {
    fn build(&self) -> BinaryExpression {
        BinaryExpression::new(
            self.name,
            self.input_types.clone(),
            self.output_type.clone(),
            self.kernel,
        )
    }
}

macro_rules! define_builtin_expressions {
    ($( $name:literal => ($left:expr, $right:expr) -> $output:expr => $kernel:path ),+ $(,)?) => {
        pub const BUILTIN_EXPRESSION_NAMES: &[&str] = &[$($name),+];

        const BUILTIN_EXPRESSIONS: &[BinaryBuiltin] = &[
            $(BinaryBuiltin {
                name: $name,
                input_types: [$left, $right],
                output_type: $output,
                kernel: $kernel,
            }),+
        ];

        pub fn build_builtin_expression(name: &str) -> Option<Box<dyn Expression>> {
            BUILTIN_EXPRESSIONS
                .iter()
                .find(|builtin| builtin.name == name)
                .map(|builtin| Box::new(builtin.build()) as Box<dyn Expression>)
        }
    };
}

define_builtin_expressions! {
    "i32_add" => (crate::PhysicalType::Int32, crate::PhysicalType::Int32)
        -> crate::PhysicalType::Int32 => evaluate_i32_add_batch,
    "string_concat" => (crate::PhysicalType::String, crate::PhysicalType::String)
        -> crate::PhysicalType::String => evaluate_string_concat_batch,
}
