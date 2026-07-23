// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Expression framework based on array

mod impl_;
mod registry;

use expr_common::datatype::DataType;
use expr_common::expr::Expression;
use expr_macro_rules::datatype_macros::*;
use expr_template::BinaryExpression;
pub use registry::{BindError, BoundExpression, FunctionRegistry};

/// All supported expression functions
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionFunc {
    Add,
    CmpLe,
    CmpGe,
    CmpEq,
    CmpNe,
    StrContains,
}

/// Numeric combinations supported by the generic arithmetic kernel.
macro_rules! for_all_arithmetic_combinations {
    ($macro:ident $(, $x:ident)*) => {
        $macro! {
            [$($x),*],
            { int16, int16, int16 },
            { int32, int32, int32 },
            { int64, int64, int64 },
            { float32, float32, float32 },
            { float64, float64, float64 },
            { int16, int32, int32 },
            { int32, int16, int32 },
            { int16, int64, int64 },
            { int64, int16, int64 },
            { int32, int64, int64 },
            { int64, int32, int64 },
            { float32, float64, float64 },
            { float64, float32, float64 },
            { int16, float32, float32 },
            { float32, int16, float32 },
            { int32, float32, float64 },
            { float32, int32, float64 },
            { int16, float64, float64 },
            { float64, int16, float64 },
            { int32, float64, float64 },
            { float64, int32, float64 }
        }
    };
}

macro_rules! logical_type_of {
    (int16) => {
        DataType::SmallInt
    };
    (int32) => {
        DataType::Integer
    };
    (int64) => {
        DataType::BigInt
    };
    (float32) => {
        DataType::Real
    };
    (float64) => {
        DataType::Double
    };
}

/// Composes all combinations of possible comparisons
///
/// Each item in the list `{ a, b, c }` represents:
/// * 1st position: left input type.
/// * 2nd position: right input type.
/// * 3rd position: cast type. For example, we need to cast the left i32 to i64 before comparing i32
///   and i64.
macro_rules! for_all_cmp_combinations {
    ($macro:ident $(, $x:ident)*) => {
        $macro! {
            [$($x),*],
            // comparison for the same type
            { int16, int16, int16 },
            { int32, int32, int32 },
            { int64, int64, int64 },
            { float32, float32, float32 },
            { float64, float64, float64 },
            { decimal, decimal, decimal },
            { fwchar, fwchar, fwchar },
            { varchar, varchar, varchar },
            { fwchar, varchar, varchar },
            { varchar, fwchar, varchar },
            // comparison across integer types
            { int16, int32, int32 },
            { int32, int16, int32 },
            { int16, int64, int64 },
            { int32, int64, int64 },
            { int64, int16, int64 },
            { int64, int32, int64 },
            // comparison across float types
            { float32, float64, float64 },
            { float64, float32, float64 },
            // comparison across integer and float32 types
            { int16, float32, float32 },
            { float32, int16, float32 },
            { int32, float32, float64 },
            { float32, int32, float64 },
            // comparison across integer and float64 types
            { int32, float64, float64 },
            { float64, int32, float64 },
            { int16, float64, float64 },
            { float64, int16, float64 },
            // comparison with decimal types
            { int16, decimal, decimal },
            { decimal, int16, decimal },
            { int32, decimal, decimal },
            { decimal, int32, decimal },
            { int64, decimal, decimal },
            { decimal, int64, decimal }
        }
    };
}

/// Generate all variants of comparison expressions
macro_rules! impl_cmp_expression_of {
    ([$i1t:ident, $i2t:ident, $cmp_func:ident, $function:ident], $({ $i1:ident, $i2:ident, $convert:ident }),*) => {
        match ($i1t, $i2t) {
            $(
                // Here we want to fill a match pattern. For example, `DataType::SmallInt` or
                // `DataType::Decimal { precision: _, .. }`. The `datatype_match_pattern` macro
                // could help us extract the pattern from `$i1` macro. Therefore, we can use
                // `$i1! { datatype_match_pattern }` to get something like
                // `DataType::Decimal { precision: _, .. }`.
                ($i1! { datatype_match_pattern }, $i2! { datatype_match_pattern }) => {
                    // Here we want to build BinaryExpression::<InputArray1, InputArray2, OutputArray>.
                    // Hence, we use `$i1! { datatype_array }` to get `InputArray1`.
                    // `$i1! { datatype_array }` will generate something like `I32Array`.
                    Ok(Box::new(BinaryExpression::<
                        $i1! { datatype_scalar },
                        $i2! { datatype_scalar },
                        bool,
                        _
                    >::new(
                        // Here we want to build CmpFunc::<InputArray1, InputArray2, CastArray>.
                        // So we use `$convert! { datatype_array }` to get cast array type.
                        // `$convert! { datatype_array }` will generate something like `I32Array`.
                        $cmp_func::<
                            $i1! { datatype_scalar },
                            $i2! { datatype_scalar },
                            $convert! { datatype_scalar }
                        >,
                    )) as Box<dyn Expression>)
                }
            )*
            (left, right) => Err(BindError::UnsupportedArguments {
                function: $function,
                left,
                right,
            })
        }
    };
}

macro_rules! impl_arithmetic_expression_of {
    ([$i1t:ident, $i2t:ident, $function:ident], $({ $i1:ident, $i2:ident, $output:ident }),*) => {
        match ($i1t, $i2t) {
            $(
                ($i1! { datatype_match_pattern }, $i2! { datatype_match_pattern }) => {
                    Ok((
                        Box::new(BinaryExpression::<
                            $i1! { datatype_scalar },
                            $i2! { datatype_scalar },
                            $output! { datatype_scalar },
                            _
                        >::new(
                            impl_::arithmetic::add::<
                                $i1! { datatype_scalar },
                                $i2! { datatype_scalar },
                                $output! { datatype_scalar }
                            >,
                        )) as Box<dyn Expression>,
                        logical_type_of!($output),
                    ))
                }
            )*
            (left, right) => Err(BindError::UnsupportedArguments {
                function: $function,
                left,
                right,
            })
        }
    };
}

fn bind_arithmetic(
    function: ExpressionFunc,
    left: DataType,
    right: DataType,
) -> Result<(Box<dyn Expression>, DataType), BindError> {
    match function {
        ExpressionFunc::Add => {
            for_all_arithmetic_combinations! { impl_arithmetic_expression_of, left, right, function }
        }
        _ => unreachable!(),
    }
}

fn bind_comparison(
    function: ExpressionFunc,
    left: DataType,
    right: DataType,
) -> Result<Box<dyn Expression>, BindError> {
    use ExpressionFunc::*;
    use impl_::cmp::*;

    match function {
        CmpLe => {
            for_all_cmp_combinations! { impl_cmp_expression_of, left, right, cmp_le, function }
        }
        CmpGe => {
            for_all_cmp_combinations! { impl_cmp_expression_of, left, right, cmp_ge, function }
        }
        CmpEq => {
            for_all_cmp_combinations! { impl_cmp_expression_of, left, right, cmp_eq, function }
        }
        CmpNe => {
            for_all_cmp_combinations! { impl_cmp_expression_of, left, right, cmp_ne, function }
        }
        Add | StrContains => unreachable!(),
    }
}

/// Check logical input types and select one concrete vectorized kernel.
pub fn bind_binary_expression(
    function: ExpressionFunc,
    left: DataType,
    right: DataType,
) -> Result<BoundExpression, BindError> {
    use impl_::string::str_contains;

    let (expression, output_type) = match function {
        ExpressionFunc::Add => bind_arithmetic(function, left.clone(), right.clone())?,
        ExpressionFunc::CmpLe
        | ExpressionFunc::CmpGe
        | ExpressionFunc::CmpEq
        | ExpressionFunc::CmpNe => (
            bind_comparison(function, left.clone(), right.clone())?,
            DataType::Boolean,
        ),
        ExpressionFunc::StrContains
            if left.physical_type() == expr_common::array::PhysicalType::String
                && right.physical_type() == expr_common::array::PhysicalType::String =>
        {
            (
                Box::new(BinaryExpression::<String, String, bool, _>::new(
                    str_contains,
                )) as Box<dyn Expression>,
                DataType::Boolean,
            )
        }
        ExpressionFunc::StrContains => {
            return Err(BindError::UnsupportedArguments {
                function,
                left,
                right,
            });
        }
    };

    Ok(BoundExpression::new(expression, [left, right], output_type))
}

/// Build an expression with runtime information.
///
/// This compatibility helper retains the original API. New code should use
/// [`bind_binary_expression`] or [`FunctionRegistry::bind_binary`] and handle [`BindError`].
pub fn build_binary_expression(
    f: ExpressionFunc,
    i1: DataType,
    i2: DataType,
) -> Box<dyn Expression> {
    bind_binary_expression(f, i1, i2)
        .expect("unsupported binary expression signature")
        .into_expression()
}

#[cfg(test)]
mod tests {
    use expr_common::array::{Array, F64Array, I16Array, I32Array, StringArray};
    use expr_common::column::ColumnViewImpl;
    use expr_common::scalar::ScalarRefImpl;

    use super::*;

    #[test]
    fn test_build_str_contains() {
        let expr = build_binary_expression(
            ExpressionFunc::StrContains,
            DataType::Varchar,
            DataType::Char { width: 10 },
        );

        for _ in 0..10 {
            let result = expr
                .eval_expr(&[
                    &StringArray::from_slice(&[Some("000"), Some("111"), None]).into(),
                    &StringArray::from_slice(&[Some("0"), Some("0"), None]).into(),
                ])
                .unwrap();
            assert_eq!(result.get(0).unwrap(), ScalarRefImpl::Bool(true));
            assert_eq!(result.get(1).unwrap(), ScalarRefImpl::Bool(false));
            assert!(result.get(2).is_none());
        }
    }

    #[test]
    fn test_cmp_i16_f64() {
        let expr =
            build_binary_expression(ExpressionFunc::CmpGe, DataType::SmallInt, DataType::Double);

        let result = expr
            .eval_expr(&[
                &I16Array::from_slice(&[Some(1), Some(2), None]).into(),
                &F64Array::from_slice(&[Some(0.0), Some(3.0), None]).into(),
            ])
            .unwrap();
        assert_eq!(result.get(0).unwrap(), ScalarRefImpl::Bool(true));
        assert_eq!(result.get(1).unwrap(), ScalarRefImpl::Bool(false));
    }

    #[test]
    fn registry_rejects_invalid_custom_function_signature() {
        let registry = FunctionRegistry::with_builtins();
        let error = registry
            .bind_binary("contains", DataType::Integer, DataType::Varchar)
            .err()
            .unwrap();
        assert!(matches!(
            error,
            BindError::UnsupportedArguments {
                function: ExpressionFunc::StrContains,
                ..
            }
        ));
    }

    #[test]
    fn bound_expression_reads_dictionary_and_constant_views() {
        let registry = FunctionRegistry::with_builtins();
        let expression = registry
            .bind_binary("contains", DataType::Varchar, DataType::Varchar)
            .unwrap();
        assert_eq!(expression.output_type(), &DataType::Boolean);

        let dictionary =
            StringArray::from_slice(&[Some("rust"), Some("database"), Some("type system")]).into();
        let indices = [Some(0), Some(1), None, Some(2)];
        let left = ColumnViewImpl::dictionary(&indices, &dictionary).unwrap();
        let right = ColumnViewImpl::constant(ScalarRefImpl::String("a"), indices.len());

        let result = expression.eval(&[left, right]).unwrap();
        assert_eq!(result.get(0), Some(ScalarRefImpl::Bool(false)));
        assert_eq!(result.get(1), Some(ScalarRefImpl::Bool(true)));
        assert_eq!(result.get(2), None);
        assert_eq!(result.get(3), Some(ScalarRefImpl::Bool(false)));
    }

    #[test]
    fn comparison_names_include_equality() {
        let expression = bind_binary_expression(
            ExpressionFunc::CmpGe,
            DataType::SmallInt,
            DataType::SmallInt,
        )
        .unwrap();
        let left = I16Array::from_slice(&[Some(1)]).into();
        let right = I16Array::from_slice(&[Some(1)]).into();
        let result = expression.eval_arrays(&[&left, &right]).unwrap();
        assert_eq!(result.get(0), Some(ScalarRefImpl::Bool(true)));
    }

    #[test]
    fn numeric_addition_promotes_inputs_during_binding() {
        let registry = FunctionRegistry::with_builtins();
        let expression = registry
            .bind_binary("+", DataType::SmallInt, DataType::Integer)
            .unwrap();
        assert_eq!(expression.output_type(), &DataType::Integer);

        let left = I16Array::from_slice(&[Some(1), None, Some(3)]).into();
        let right = I32Array::from_slice(&[Some(10), Some(20), Some(30)]).into();
        let result = expression.eval_arrays(&[&left, &right]).unwrap();
        assert_eq!(result.get(0), Some(ScalarRefImpl::Int32(11)));
        assert_eq!(result.get(1), None);
        assert_eq!(result.get(2), Some(ScalarRefImpl::Int32(33)));
    }
}
