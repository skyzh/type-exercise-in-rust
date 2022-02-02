// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

#![allow(unused_macros)]
#![allow(unused_imports)]

//! Necessary macros to associated logical data type to array types
//!
//! All macros in this module will pass two parameters to the parent:
//!
//! * 1st position: DataType enum match pattern
//! * 2st position: Array type
//! * 3rd position: Scalar type
//!
//! Developers can use `datatype_match_pattern` and `datatype_array` to extract information from
//! data types' macros.

/// Get the type match pattern out of the type macro. e.g., `DataTypeKind::Decimal { .. }`.
macro_rules! datatype_match_pattern {
    ($match_pattern:pat, $array:ty, $scalar:ty) => {
        $match_pattern
    };
}

pub(crate) use datatype_match_pattern;

/// Get the array type out of the type macro. e.g., `Int32Array`.
macro_rules! datatype_array {
    ($match_pattern:pat, $array:ty, $scalar:ty) => {
        $array
    };
}

pub(crate) use datatype_array;

/// Get the scalar type out of the type macro. e.g., `i32`.
macro_rules! datatype_scalar {
    ($match_pattern:pat, $array:ty, $scalar:ty) => {
        $scalar
    };
}

pub(crate) use datatype_scalar;

/// Association information for `Boolean` logical type.
macro_rules! boolean {
    ($macro:ident) => {
        $macro! {
            DataType::Boolean,
            BoolArray,
            bool
        }
    };
}

pub(crate) use boolean;

/// Association information for `SmallInt` logical type.
macro_rules! int16 {
    ($macro:ident) => {
        $macro! {
            DataType::SmallInt,
            I16Array,
            i16
        }
    };
}

pub(crate) use int16;

/// Association information for `Integer` logical type.
macro_rules! int32 {
    ($macro:ident) => {
        $macro! {
            DataType::Integer,
            I32Array,
            i32
        }
    };
}

pub(crate) use int32;

/// Association information for `BigInt` logical type.
macro_rules! int64 {
    ($macro:ident) => {
        $macro! {
            DataType::BigInt,
            I64Array,
            i64
        }
    };
}

pub(crate) use int64;

/// Association information for `Varchar` logical type.
macro_rules! varchar {
    ($macro:ident) => {
        $macro! {
            DataType::Varchar,
            StringArray,
            String
        }
    };
}

pub(crate) use varchar;

/// Association information for `Char` logical type.
macro_rules! fwchar {
    ($macro:ident) => {
        $macro! {
            DataType::Char { .. },
            StringArray,
            String
        }
    };
}

pub(crate) use fwchar;

/// Association information for `Real` logical type.
macro_rules! float32 {
    ($macro:ident) => {
        $macro! {
            DataType::Real,
            F32Array,
            f32
        }
    };
}

pub(crate) use float32;

/// Association information for `Real` logical type.
macro_rules! float64 {
    ($macro:ident) => {
        $macro! {
            DataType::Double,
            F64Array,
            f64
        }
    };
}

pub(crate) use float64;

/// Association information for `Decimal` logical type.
macro_rules! decimal {
    ($macro:ident) => {
        $macro! {
            DataType::Decimal { .. },
            DecimalArray,
            rust_decimal::Decimal
        }
    };
}

pub(crate) use decimal;
