use crate::variant_catalog::for_each_physical_family;

/// Checkpoint 1: extend this starter pair to every non-List physical type.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalType {
    Int32,
    String,
    // Add Int16, Int64, Bool, Float32, Float64, and `Decimal(DecimalType)`.
    // A later checkpoint adds List.
}

/// Checkpoint 1: extend this descriptor-free inventory in the same order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalFamily {
    Int32,
    String,
    // Add Int16, Int64, Bool, Float32, Float64, and Decimal.
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

/// Checkpoint 1: use this value for checked erased-to-typed failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMismatch {
    pub expected: PhysicalType,
    pub actual: PhysicalType,
}

// Checkpoint 1: implement `Display` and `std::error::Error` for `TypeMismatch`.
