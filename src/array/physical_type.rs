#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub enum PhysicalType {
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    Bool,
    String,
    Decimal,
    List,
}
