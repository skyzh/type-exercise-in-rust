/// Day 2, checkpoint 2: implement the single inventory of supported non-List families.
macro_rules! for_each_physical_family {
    ($callback:ident) => {
        $callback! {
            { copy, Int16, I16Array, I16ArrayBuilder, i16, i16 },
            { copy, Int32, I32Array, I32ArrayBuilder, i32, i32 },
            { copy, Int64, I64Array, I64ArrayBuilder, i64, i64 },
            { copy, Bool, BoolArray, BoolArrayBuilder, bool, bool },
            { copy, Float32, F32Array, F32ArrayBuilder, f32, f32 },
            { copy, Float64, F64Array, F64ArrayBuilder, f64, f64 },
            { borrowed, String, StringArray, StringArrayBuilder, String, &'a str },
            { decimal, Decimal, DecimalArray, DecimalArrayBuilder, Decimal, Decimal },
        }
    };
}

pub(crate) use for_each_physical_family;
