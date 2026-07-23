// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Implements logical types for a database system

use crate::array::PhysicalType;

/// Encapsules all supported (logical) data types in the system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataType {
    /// Corresponding to Int16 physical type
    SmallInt,
    /// Corresponding to Int32 physical type
    Integer,
    /// Corresponding to Int64 physical type
    BigInt,
    /// Corresponding to String physical type
    Varchar,
    /// Corresponding to String physical type
    Char { width: u16 },
    /// Corresponding to Bool physical type
    Boolean,
    /// Corresponding to Float32 physical type
    Real,
    /// Corresponding to Float64 physical type
    Double,
    /// Corresponding to Decimal physical type
    Decimal { scale: u16, precision: u16 },
}

impl DataType {
    /// Map a logical SQL type to the physical scalar/array representation used at runtime.
    pub fn physical_type(&self) -> PhysicalType {
        match self {
            Self::SmallInt => PhysicalType::Int16,
            Self::Integer => PhysicalType::Int32,
            Self::BigInt => PhysicalType::Int64,
            Self::Varchar | Self::Char { .. } => PhysicalType::String,
            Self::Boolean => PhysicalType::Bool,
            Self::Real => PhysicalType::Float32,
            Self::Double => PhysicalType::Float64,
            Self::Decimal { .. } => PhysicalType::Decimal,
        }
    }
}
