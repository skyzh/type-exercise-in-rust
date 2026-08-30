use anyhow::Result;

use crate::{DecimalType, PhysicalType};

/// A planner-visible type that maps to one physical scalar and array family.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DataType {
    SmallInt,
    Integer,
    BigInt,
    Boolean,
    Real,
    Double,
    Varchar,
    Char { width: u16 },
    Decimal(DecimalType),
    List(Box<DataType>),
}

impl DataType {
    pub fn physical_type(&self) -> PhysicalType {
        match self {
            Self::SmallInt => PhysicalType::Int16,
            Self::Integer => PhysicalType::Int32,
            Self::BigInt => PhysicalType::Int64,
            Self::Boolean => PhysicalType::Bool,
            Self::Real => PhysicalType::Float32,
            Self::Double => PhysicalType::Float64,
            Self::Varchar | Self::Char { .. } => PhysicalType::String,
            Self::Decimal(decimal_type) => PhysicalType::Decimal(*decimal_type),
            Self::List(element_type) => PhysicalType::List(Box::new(element_type.physical_type())),
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::Varchar | Self::Char { .. })
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::SmallInt
                | Self::Integer
                | Self::BigInt
                | Self::Real
                | Self::Double
                | Self::Decimal(_)
        )
    }

    pub fn decimal(precision: u8, scale: u8) -> Result<Self> {
        DecimalType::try_new(precision, scale).map(Self::Decimal)
    }
}
