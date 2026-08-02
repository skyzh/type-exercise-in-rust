//! Implement Debug for Array

use super::Array;

/// Debug format an array with range
pub fn debug_fmt_ranged(
    array: &impl Array,
    from: usize,
    to: usize,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    write!(f, "[")?;
    let len = array.len();
    for (idx, item) in array.iter().enumerate().skip(from).take(to - from) {
        write!(f, "{:?}", item)?;
        if idx != len - 1 {
            write!(f, ", ")?;
        }
    }
    write!(f, "]")?;
    Ok(())
}

/// Debug format an array
pub fn debug_fmt(array: &impl Array, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    debug_fmt_ranged(array, 0, array.len(), f)
}
