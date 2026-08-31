//! Scalar comparison operations and their typed expression adapters.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonOperator {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

pub(crate) fn less<T: PartialOrd>(left: T, right: T) -> bool {
    left < right
}

pub(crate) fn less_or_equal<T: PartialOrd>(left: T, right: T) -> bool {
    left <= right
}

pub(crate) fn greater<T: PartialOrd>(left: T, right: T) -> bool {
    left > right
}

pub(crate) fn greater_or_equal<T: PartialOrd>(left: T, right: T) -> bool {
    left >= right
}

pub(crate) fn equal<T: PartialEq>(left: T, right: T) -> bool {
    left == right
}

pub(crate) fn not_equal<T: PartialEq>(left: T, right: T) -> bool {
    left != right
}
