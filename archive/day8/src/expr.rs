// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Expression framework based on array

use anyhow::Result;

use self::vectorize::BinaryExpression;
use crate::array::ArrayImpl;
use crate::datatype::macros::*;
use crate::datatype::DataType;

mod cmp;
mod string;
mod vectorize;

/// A trait over all expressions -- unary, binary, etc.
pub trait Expression {
    /// Evaluate an expression with run-time number of [`ArrayImpl`]s.
    fn eval_expr(&self, data: &[&ArrayImpl]) -> Result<ArrayImpl>;
}

/// All supported expression functions
pub enum ExpressionFunc {
    CmpLe,
    CmpGe,
    CmpEq,
    CmpNe,
    StrContains,
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
    ([$i1t:ident, $i2t:ident, $cmp_func:ident], $({ $i1:ident, $i2:ident, $convert:ident }),*) => {
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
                    Box::new(BinaryExpression::<
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
                    ))
                }
            )*
            (other_dt1, other_dt2) => unimplemented!("unsupported comparison: {:?} <{}> {:?}",
                other_dt1,
                stringify!($cmp_func),
                other_dt2)
        }
    };
}

/// Build expression with runtime information.
pub fn build_binary_expression(
    f: ExpressionFunc,
    i1: DataType,
    i2: DataType,
) -> Box<dyn Expression> {
    use ExpressionFunc::*;

    use crate::expr::cmp::*;
    use crate::expr::string::*;

    match f {
        CmpLe => for_all_cmp_combinations! { impl_cmp_expression_of, i1, i2, cmp_le },
        CmpGe => for_all_cmp_combinations! { impl_cmp_expression_of, i1, i2, cmp_ge },
        CmpEq => for_all_cmp_combinations! { impl_cmp_expression_of, i1, i2, cmp_eq },
        CmpNe => for_all_cmp_combinations! { impl_cmp_expression_of, i1, i2, cmp_ne },
        StrContains => Box::new(BinaryExpression::<String, String, bool, _>::new(
            str_contains,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::{Array, F64Array, I16Array, StringArray};
    use crate::scalar::ScalarRefImpl;

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
}
