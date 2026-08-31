use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::DecimalType;

/// The exact physical representation selected at a runtime boundary.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalType {
    Int16,
    Int32,
    Int64,
    Bool,
    Float32,
    Float64,
    String,
    Decimal(DecimalType),
    // Day 12 adds `List(Box<PhysicalType>)`.
}

/// A descriptor-free family tag used only for catalog completeness.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalFamily {
    Int16,
    Int32,
    Int64,
    Bool,
    Float32,
    Float64,
    String,
    Decimal,
}

/// One catalog row used to audit the single physical-family definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalFamilyEntry {
    pub family: PhysicalFamily,
    pub name: &'static str,
}

/// Every supported non-List physical family, in catalog order.
pub const PHYSICAL_FAMILY_CATALOG: &[PhysicalFamilyEntry] = &[
    PhysicalFamilyEntry {
        family: PhysicalFamily::Int16,
        name: "Int16",
    },
    PhysicalFamilyEntry {
        family: PhysicalFamily::Int32,
        name: "Int32",
    },
    PhysicalFamilyEntry {
        family: PhysicalFamily::Int64,
        name: "Int64",
    },
    PhysicalFamilyEntry {
        family: PhysicalFamily::Bool,
        name: "Bool",
    },
    PhysicalFamilyEntry {
        family: PhysicalFamily::Float32,
        name: "Float32",
    },
    PhysicalFamilyEntry {
        family: PhysicalFamily::Float64,
        name: "Float64",
    },
    PhysicalFamilyEntry {
        family: PhysicalFamily::String,
        name: "String",
    },
    PhysicalFamilyEntry {
        family: PhysicalFamily::Decimal,
        name: "Decimal",
    },
];

/// A checked erased-to-typed conversion found the wrong physical representation.
#[derive(Clone, Debug, Eq, PartialEq)]
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
