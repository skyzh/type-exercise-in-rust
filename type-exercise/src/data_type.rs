use crate::PhysicalType;

/// A planner-visible type that maps to one physical scalar and array family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DataType {
    Integer,
    Varchar,
    Char { width: u16 },
}

impl DataType {
    pub fn physical_type(self) -> PhysicalType {
        match self {
            Self::Integer => PhysicalType::Int32,
            Self::Varchar | Self::Char { .. } => PhysicalType::String,
        }
    }

    pub fn is_string(self) -> bool {
        matches!(self, Self::Varchar | Self::Char { .. })
    }
}
