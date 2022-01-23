// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Necessary macros to associated logical data type to array types
//!
//! All macros in this module will pass two parameters to the parent:
//!
//! * 1st position: DataType enum match pattern
//! * 2st position: Array type
//!
//! Developers can use `datatype_match_pattern` and `datatype_array` to extract information from
//! data types' macros.

/// Get the type match pattern out of the type macro. e.g., `DataTypeKind::Decimal { .. }`.
macro_rules! datatype_match_pattern {
    ($match_pattern:pat, $array:ty) => {
        $match_pattern
    };
}

pub(crate) use datatype_match_pattern;

/// Get the array type out of the type macro. e.g., `Int32Array`.
macro_rules! datatype_array {
    ($match_pattern:pat, $array:ty) => {
        $array
    };
}

pub(crate) use datatype_array;

/// Association information for `Boolean` logical type.
#[allow(unused_macros)]
macro_rules! boolean {
    ($macro:tt) => {
        $macro! {
            DataType::Boolean,
            BoolArray
        }
    };
}

#[allow(unused_imports)]
pub(crate) use boolean;

/// Association information for `SmallInt` logical type.
macro_rules! int16 {
    ($macro:tt) => {
        $macro! {
            DataType::SmallInt,
            I16Array
        }
    };
}

pub(crate) use int16;

/// Association information for `Integer` logical type.
macro_rules! int32 {
    ($macro:tt) => {
        $macro! {
            DataType::Integer,
            I32Array
        }
    };
}

pub(crate) use int32;

/// Association information for `BigInt` logical type.
macro_rules! int64 {
    ($macro:tt) => {
        $macro! {
            DataType::BigInt,
            I64Array
        }
    };
}

pub(crate) use int64;

/// Association information for `Varchar` logical type.
macro_rules! varchar {
    ($macro:tt) => {
        $macro! {
            DataType::Varchar,
            StringArray
        }
    };
}

pub(crate) use varchar;

/// Association information for `Char` logical type.
macro_rules! fwchar {
    ($macro:tt) => {
        $macro! {
            DataType::Char { .. },
            StringArray
        }
    };
}

pub(crate) use fwchar;

/// Association information for `Real` logical type.
macro_rules! float32 {
    ($macro:tt) => {
        $macro! {
            DataType::Real,
            F32Array
        }
    };
}

pub(crate) use float32;

/// Association information for `Real` logical type.
macro_rules! float64 {
    ($macro:tt) => {
        $macro! {
            DataType::Double,
            F64Array
        }
    };
}

pub(crate) use float64;

/// Association information for `Decimal` logical type.
macro_rules! decimal {
    ($macro:tt) => {
        $macro! {
            DataType::Decimal { .. },
            DecimalArray
        }
    };
}

pub(crate) use decimal;
