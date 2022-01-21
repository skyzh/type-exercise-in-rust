// Copyright 2022 RisingLight Project Authors. Licensed under Apache-2.0.

//! Necessary macros to cover variants of array types.

/// `for_all_variants` includes all variants of our array types. If you added a new array
/// type inside the project, be sure to add a variant here.
///
/// Every tuple has four elements, where
/// `{ enum variant name, function suffix name, array type, builder type, scalar type }`
macro_rules! for_all_variants {
    ($macro:tt $(, $x:tt)*) => {
        $macro! {
            [$($x),*],
            { Int32, int32, I32Array, I32ArrayBuilder, i32, i32 },
            // { Int64, int64, I64Array, I64ArrayBuilder, Int64 },
            { Float32, float32, F32Array, F32ArrayBuilder, f32, f32 },
            { String, string, StringArray, StringArrayBuilder, String, &'a str }
            // { Bool, bool, BoolArray, BoolArrayBuilder, Bool }
        }
    };
}

pub(crate) use for_all_variants;
