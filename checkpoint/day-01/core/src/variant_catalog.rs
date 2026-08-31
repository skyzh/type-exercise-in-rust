/// Invoke a callback with the two physical families completed on Day 1.
macro_rules! for_each_physical_family {
    ($callback:ident) => {
        $callback! {
            { copy, Int32, I32Array, I32ArrayBuilder, i32, i32 },
            { borrowed, String, StringArray, StringArrayBuilder, String, &'a str },
        }
    };
}

pub(crate) use for_each_physical_family;
