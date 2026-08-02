// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

// Copyright 2022 RisingLight Project Authors. Licensed under Apache-2.0.

//! Necessary macros to cover variants of array types.

/// `for_all_variants` includes all variants of our array types. If you added a new array
/// type inside the project, be sure to add a variant here.
///
/// Every tuple has four elements, where
/// `{ enum variant name, function suffix name, array type, builder type, scalar type }`
macro_rules! for_all_variants {
    ($macro:ident $(, $x:ident)*) => {
        $macro! {
            [$($x),*],
            { Int16, int16, I16Array, I16ArrayBuilder, i16, i16 },
            { Int32, int32, I32Array, I32ArrayBuilder, i32, i32 },
            { Int64, int64, I64Array, I64ArrayBuilder, i64, i64 },
            { Float32, float32, F32Array, F32ArrayBuilder, f32, f32 },
            { Float64, float64, F64Array, F64ArrayBuilder, f64, f64 },
            { Bool, bool, BoolArray, BoolArrayBuilder, bool, bool },
            { String, string, StringArray, StringArrayBuilder, String, &'a str },
            { Decimal, decimal, DecimalArray, DecimalArrayBuilder, Decimal, Decimal },
            { List, list, ListArray, ListArrayBuilder, List, ListRef<'a> }
        }
    };
}

pub(crate) use for_all_variants;

macro_rules! for_all_primitive_variants {
    ($macro:ident $(, $x:ident)*) => {
        $macro! {
            [$($x),*],
            { Int16, int16, I16Array, I16ArrayBuilder, i16, i16 },
            { Int32, int32, I32Array, I32ArrayBuilder, i32, i32 },
            { Int64, int64, I64Array, I64ArrayBuilder, i64, i64 },
            { Float32, float32, F32Array, F32ArrayBuilder, f32, f32 },
            { Float64, float64, F64Array, F64ArrayBuilder, f64, f64 },
            { Bool, bool, BoolArray, BoolArrayBuilder, bool, bool },
            { Decimal, decimal, DecimalArray, DecimalArrayBuilder, Decimal, Decimal }
        }
    };
}
pub(crate) use for_all_primitive_variants;
