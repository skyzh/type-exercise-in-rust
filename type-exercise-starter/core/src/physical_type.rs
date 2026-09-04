use crate::variant_catalog::for_each_physical_family;

/// The two physical types visible at the start of Chapter 1.
///
/// Chapter 1 adds Int16, Int64, Bool, Float32, Float64, and Decimal. Chapter 9 adds List.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalType {
    Int32,
    String,
    // Chapter 1: add the remaining primitive variants and `Decimal(DecimalType)`.
    // Chapter 9: add `List(Box<PhysicalType>)`.
}

/// The two descriptor-free families visible at the start of Chapter 1.
///
/// Chapter 1 extends this inventory with the remaining non-List families.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PhysicalFamily {
    Int32,
    String,
    // Chapter 1: add Int16, Int64, Bool, Float32, Float64, and Decimal.
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

/// Chapter 1: use this value for checked erased-to-typed failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMismatch {
    pub expected: PhysicalType,
    pub actual: PhysicalType,
}

// Chapter 1: implement `Display` and `std::error::Error` for `TypeMismatch`.
