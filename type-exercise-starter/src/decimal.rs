use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::PhysicalType;

/// Day 2, checkpoint 4: implement one checked precision/scale descriptor shared by a column.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DecimalType {
    precision: u8,
    scale: u8,
}

impl DecimalType {
    pub const MAX_PRECISION: u8 = 38;
    pub fn try_new(_: u8, _: u8) -> Result<Self, DecimalError> {
        todo!("validate Decimal metadata in Day 2")
    }
    pub fn precision(self) -> u8 {
        todo!("expose Decimal precision in Day 2")
    }
    pub fn scale(self) -> u8 {
        todo!("expose Decimal scale in Day 2")
    }
}

/// Day 2, checkpoint 4: implement a transient typed scalar, never per-row array metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decimal {
    unscaled: i128,
    decimal_type: DecimalType,
}

impl Decimal {
    pub fn try_new(_: i128, _: DecimalType) -> Result<Self, DecimalError> {
        todo!("validate a Decimal scalar in Day 2")
    }
    pub fn unscaled(self) -> i128 {
        todo!("expose the unscaled coefficient in Day 2")
    }
    pub fn decimal_type(self) -> DecimalType {
        todo!("preserve shared Decimal metadata in Day 2")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Day 2, checkpoint 4: return readable errors for invalid Decimal metadata and rows.
pub enum DecimalError {
    InvalidPrecision {
        precision: u8,
    },
    ScaleExceedsPrecision {
        precision: u8,
        scale: u8,
    },
    CoefficientOutOfRange {
        decimal_type: DecimalType,
        unscaled: i128,
    },
    MetadataMismatch {
        expected: DecimalType,
        actual: DecimalType,
    },
    ValueValidityLength {
        values: usize,
        validity: usize,
    },
    ExpectedDecimal {
        actual: PhysicalType,
    },
}

impl Display for DecimalError {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        todo!("format Decimal errors in Day 2")
    }
}

impl Error for DecimalError {}
