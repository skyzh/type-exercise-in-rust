use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::DecimalType;
use crate::variant_catalog::for_each_physical_family;

/// Day 1, checkpoint 1: implement the Int32 and String physical rows.
/// Day 2, checkpoints 1–4: extend this enum with the remaining scalar rows and Decimal metadata.
/// Day 11, checkpoint 1: extend it with `// List(Box<PhysicalType>),`.
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
}

/// Day 2, checkpoint 2: implement the descriptor-free family inventory used by the catalog.
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
    ($( { $kind:ident, $variant:ident, $array:ident, $builder:ident, $owned:ty, $borrowed:ty } ),+ $(,)?) => {
        pub const PHYSICAL_FAMILY_CATALOG: &[PhysicalFamilyEntry] = &[
            $(PhysicalFamilyEntry {
                family: PhysicalFamily::$variant,
                name: stringify!($variant),
            }),+
        ];
    };
}

for_each_physical_family!(define_family_catalog);

/// Day 1, checkpoint 1: implement a checked erased-to-typed conversion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMismatch {
    pub expected: PhysicalType,
    pub actual: PhysicalType,
}

impl Display for TypeMismatch {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        todo!("format TypeMismatch in Day 1")
    }
}

impl Error for TypeMismatch {}
