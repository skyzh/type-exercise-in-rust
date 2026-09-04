/// Chapter 1: replace the empty callback with the Int32 and String rows.
/// Chapter 1: extend that inventory with every remaining non-List family.
macro_rules! for_each_physical_family {
    ($callback:ident) => {
        $callback! {}
    };
}

pub(crate) use for_each_physical_family;
