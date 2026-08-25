use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::variant_catalog::for_each_physical_family;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalType {
    Int32,
    String,
    // Day 2 adds the remaining primitive families and Decimal.
    // Day 11 adds List.
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalFamily {
    Int32,
    String,
    // Day 2 adds the remaining non-List families.
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
