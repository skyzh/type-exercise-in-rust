/// Checkpoint 1: replace the empty callback with every non-List physical family.
macro_rules! for_each_physical_family {
    ($callback:ident) => {
        $callback! {}
    };
}

pub(crate) use for_each_physical_family;
