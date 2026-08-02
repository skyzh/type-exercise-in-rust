use std::error::Error;
use std::fmt::{Display, Formatter};

/// The physical representation selected at a runtime boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalType {
    Int32,
    String,
}

/// A checked erased-to-typed conversion found the wrong physical representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeMismatch {
    pub expected: PhysicalType,
    pub actual: PhysicalType,
}

impl Display for TypeMismatch {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "type mismatch: expected {:?}, got {:?}",
            self.expected, self.actual
        )
    }
}

impl Error for TypeMismatch {}
