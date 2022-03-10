// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

use expr_common::array::Array;
use expr_common::scalar::Scalar;
use expr_template::BinaryExpression;

fn test_if_impl<A: Scalar, B: Scalar, O: Scalar, F: Fn(A::RefType<'_>, B::RefType<'_>) -> O>(_: F) {
}

fn binary_str(_: &str, _: &str) -> String {
    todo!()
}

fn binary_generics<A: Scalar, B: Scalar, O: Scalar>(_: A::RefType<'_>, _: B::RefType<'_>) -> O {
    todo!()
}

#[test]
fn test_simple_str_function() {
    // FIXME: test_if_impl(binary_str)
    //
    // Confusing compiler error with the above line:
    // ```plain
    // error[E0631]: type mismatch in function arguments
    //   --> archive/day6-hard/src/expr/vectorize.rs:99:22
    //    |
    // 89 |     fn binary_str(_: &str, _: &str) -> String {
    //    |     ----------------------------------------- found signature of `for<'r, 's> fn(&'r
    // str, &'s str) -> _` ...
    // 99 |         test_if_impl(binary_str)
    //    |         ------------ ^^^^^^^^^^ expected signature of `for<'r, 's> fn(<_ as
    // scalar::Scalar>::RefType<'r>, <_ as scalar::Scalar>::RefType<'s>) -> _`    |
    // |    |         required by a bound introduced by this call
    // ```

    test_if_impl::<String, String, String, _>(binary_str)
}

#[test]
fn test_simple_generics_function() {
    test_if_impl::<i32, f32, i64, _>(binary_generics::<i32, f32, i64>)
}

use expr_common::array::{BoolArray, I32Array, StringArray};

use super::cmp::*;
use super::string::*;

/// Test if an array has the same content as a vector
fn check_array_eq<'a, A: Array>(array: &'a A, vec: &[Option<A::RefItem<'a>>])
where
    A::RefItem<'a>: PartialEq,
{
    for (a, b) in array.iter().zip(vec.iter()) {
        assert_eq!(&a, b);
    }
}

#[test]
fn test_cmp_le() {
    // Compare two `i32` array. Cast them to `i64` before comparing.
    let expr = BinaryExpression::<i32, i32, bool, _>::new(cmp_le::<i32, i32, i64>);
    let result = expr
        .eval_batch(
            &I32Array::from_slice(&[Some(0), Some(1), None]).into(),
            &I32Array::from_slice(&[Some(1), Some(0), None]).into(),
        )
        .unwrap();
    check_array_eq::<BoolArray>(
        (&result).try_into().unwrap(),
        &[Some(true), Some(false), None],
    );
}

#[test]
fn test_cmp_ge_str() {
    let expr = BinaryExpression::<String, String, bool, _>::new(cmp_ge::<String, String, String>);
    let result = expr
        .eval_batch(
            &StringArray::from_slice(&[Some("0"), Some("1"), None]).into(),
            &StringArray::from_slice(&[Some("1"), Some("0"), None]).into(),
        )
        .unwrap();
    check_array_eq::<BoolArray>(
        (&result).try_into().unwrap(),
        &[Some(false), Some(true), None],
    );
}

#[test]
fn test_str_contains() {
    let expr = BinaryExpression::<String, String, bool, _>::new(str_contains);
    let result = expr
        .eval_batch(
            &StringArray::from_slice(&[Some("000"), Some("111"), None]).into(),
            &StringArray::from_slice(&[Some("0"), Some("0"), None]).into(),
        )
        .unwrap();
    check_array_eq::<BoolArray>(
        (&result).try_into().unwrap(),
        &[Some(true), Some(false), None],
    );
}

#[test]
fn test_str_contains_lambda() {
    let expr =
        BinaryExpression::<String, String, bool, _>::new(|x1: &str, x2: &str| x1.contains(x2));
    let result = expr
        .eval_batch(
            &StringArray::from_slice(&[Some("000"), Some("111"), None]).into(),
            &StringArray::from_slice(&[Some("0"), Some("0"), None]).into(),
        )
        .unwrap();
    check_array_eq::<BoolArray>(
        (&result).try_into().unwrap(),
        &[Some(true), Some(false), None],
    );
}
