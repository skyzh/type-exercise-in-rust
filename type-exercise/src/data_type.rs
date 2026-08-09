use crate::PhysicalType;

/// A planner-visible type that maps to one physical scalar and array family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DataType {
    SmallInt,
    Integer,
    BigInt,
    Boolean,
    Real,
    Double,
    Varchar,
    Char { width: u16 },
    Decimal { scale: u16, precision: u16 },
}

impl DataType {
    pub fn physical_type(self) -> PhysicalType {
        match self {
            Self::SmallInt => PhysicalType::Int16,
            Self::Integer => PhysicalType::Int32,
            Self::BigInt => PhysicalType::Int64,
            Self::Boolean => PhysicalType::Bool,
            Self::Real => PhysicalType::Float32,
            Self::Double => PhysicalType::Float64,
            Self::Varchar | Self::Char { .. } => PhysicalType::String,
            Self::Decimal { .. } => PhysicalType::Decimal,
        }
    }

    pub fn is_string(self) -> bool {
        matches!(self, Self::Varchar | Self::Char { .. })
    }

    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            Self::SmallInt
                | Self::Integer
                | Self::BigInt
                | Self::Real
                | Self::Double
                | Self::Decimal { .. }
        )
    }
}
