// Copyright 2022-2026 Alex Chi. Licensed under Apache-2.0.

//! Planning-time expression registry.
//!
//! The registry resolves logical input types before execution. Once a [`BoundExpression`] exists,
//! its generated kernel owns the concrete scalar types, so runtime downcasts are framework code
//! rather than a responsibility of each function author.

use std::collections::HashMap;

use expr_common::array::{ArrayImpl, PhysicalType};
use expr_common::column::ColumnViewImpl;
use expr_common::datatype::DataType;
use expr_common::expr::Expression;
use thiserror::Error;

use crate::{ExpressionFunc, bind_binary_expression};

/// An expression whose logical signature has already been checked.
pub struct BoundExpression {
    expression: Box<dyn Expression>,
    input_types: [DataType; 2],
    output_type: DataType,
}

impl BoundExpression {
    /// Assemble a bound expression from a custom typed kernel and its checked signature.
    pub fn new(
        expression: Box<dyn Expression>,
        input_types: [DataType; 2],
        output_type: DataType,
    ) -> Self {
        Self {
            expression,
            input_types,
            output_type,
        }
    }

    /// Logical input signature selected by the binder.
    pub fn input_types(&self) -> &[DataType; 2] {
        &self.input_types
    }

    /// Logical output type selected by the binder.
    pub fn output_type(&self) -> &DataType {
        &self.output_type
    }

    /// Consume the binding metadata and return the runtime expression.
    pub fn into_expression(self) -> Box<dyn Expression> {
        self.expression
    }

    /// Evaluate any compatible physical column views.
    pub fn eval(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
        for (index, (input_type, view)) in self.input_types.iter().zip(inputs).enumerate() {
            let expected = input_type.physical_type();
            let actual = view.physical_type();
            if expected != actual {
                return Err(BindError::PhysicalTypeMismatch {
                    index,
                    expected,
                    actual,
                }
                .into());
            }
        }
        self.expression.eval(inputs)
    }

    /// Compatibility adapter for regular arrays.
    pub fn eval_arrays(&self, inputs: &[&ArrayImpl]) -> anyhow::Result<ArrayImpl> {
        self.expression.eval_expr(inputs)
    }
}

/// Planning-time expression error.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum BindError {
    #[error("unknown binary expression `{0}`")]
    UnknownFunction(String),
    #[error("{function:?} does not support {left:?} and {right:?}")]
    UnsupportedArguments {
        function: ExpressionFunc,
        left: DataType,
        right: DataType,
    },
    #[error(
        "bound input {index} expects physical type {expected:?}, but the view contains {actual:?}"
    )]
    PhysicalTypeMismatch {
        index: usize,
        expected: PhysicalType,
        actual: PhysicalType,
    },
}

type BinaryFactory =
    dyn Fn(DataType, DataType) -> Result<BoundExpression, BindError> + Send + Sync + 'static;

/// A small extensible registry used during expression binding.
#[derive(Default)]
pub struct FunctionRegistry {
    binary: HashMap<String, Box<BinaryFactory>>,
}

impl FunctionRegistry {
    /// Create a registry with the course's comparison and custom string functions.
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        registry.register_binary("+", |left, right| {
            bind_binary_expression(ExpressionFunc::Add, left, right)
        });
        registry.register_binary("<=", |left, right| {
            bind_binary_expression(ExpressionFunc::CmpLe, left, right)
        });
        registry.register_binary(">=", |left, right| {
            bind_binary_expression(ExpressionFunc::CmpGe, left, right)
        });
        registry.register_binary("=", |left, right| {
            bind_binary_expression(ExpressionFunc::CmpEq, left, right)
        });
        registry.register_binary("!=", |left, right| {
            bind_binary_expression(ExpressionFunc::CmpNe, left, right)
        });
        registry.register_binary("contains", |left, right| {
            bind_binary_expression(ExpressionFunc::StrContains, left, right)
        });
        registry
    }

    /// Register a planning-time factory for a data-type-specific expression.
    pub fn register_binary(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(DataType, DataType) -> Result<BoundExpression, BindError>
        + Send
        + Sync
        + 'static,
    ) {
        self.binary.insert(name.into(), Box::new(factory));
    }

    /// Resolve a function name and logical input types into one typed runtime kernel.
    pub fn bind_binary(
        &self,
        name: &str,
        left: DataType,
        right: DataType,
    ) -> Result<BoundExpression, BindError> {
        self.binary
            .get(name)
            .ok_or_else(|| BindError::UnknownFunction(name.to_owned()))?(left, right)
    }
}
