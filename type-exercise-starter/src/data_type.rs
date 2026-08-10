use crate::{DecimalError, DecimalType, PhysicalType};

/// Day 2, checkpoint 3: implement planner-visible logical scalar types.
/// Day 11, checkpoint 1: extend this enum with `// List(Box<DataType>),`.
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
}

impl DataType {
    pub fn decimal(_: u8, _: u8) -> Result<Self, DecimalError> {
        todo!("construct checked Decimal metadata in Day 2")
    }
    pub fn physical_type(&self) -> PhysicalType {
        todo!("map logical types to physical storage in Day 2")
    }
    pub fn is_string(&self) -> bool {
        todo!("classify string logical types in Day 2")
    }
    pub fn is_numeric(&self) -> bool {
        todo!("classify numeric logical types in Day 2")
    }
}
