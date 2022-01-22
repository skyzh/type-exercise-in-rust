//! Implements logical types for a database system

/// Encapsules all supported (logical) data types in the system.
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
    Char { length: u16 },
    /// Corresponding to Bool physical type
    Bool,
    /// Corresponding to Float32 physical type
    Real,
    /// Corresponding to Float64 physical type
    Double,
    /// Not implemented for now
    Decimal { scale: u16, precision: u16 },
}
