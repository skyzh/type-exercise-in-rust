use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::DecimalType;
use crate::variant_catalog::for_each_physical_family;

/// The in-memory representation selected for a value at runtime.
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
    List(Box<PhysicalType>),
}

/// A descriptor-free tag for each supported physical family.
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalFamilyEntry {
    pub family: PhysicalFamily,
    pub name: &'static str,
}

macro_rules! define_family_catalog {
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),* $(,)?) => {
        pub const PHYSICAL_FAMILY_CATALOG: &[PhysicalFamilyEntry] = &[
            $(PhysicalFamilyEntry {
                family: PhysicalFamily::$variant,
                name: stringify!($variant),
            }),*
        ];
    };
}

for_each_physical_family!(define_family_catalog);

/// A checked erased-to-typed conversion encountered another physical family.
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
