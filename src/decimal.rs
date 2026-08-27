use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::PhysicalType;

/// The one shared logical descriptor for a Decimal scalar or array.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DecimalType {
    precision: u8,
    scale: u8,
}

impl DecimalType {
    pub const MAX_PRECISION: u8 = 38;

    pub fn try_new(precision: u8, scale: u8) -> Result<Self, DecimalError> {
        if !(1..=Self::MAX_PRECISION).contains(&precision) {
            return Err(DecimalError::InvalidPrecision { precision });
        }
        if scale > precision {
            return Err(DecimalError::ScaleExceedsPrecision { precision, scale });
        }
        Ok(Self { precision, scale })
    }

    pub fn precision(self) -> u8 {
        self.precision
    }

    pub fn scale(self) -> u8 {
        self.scale
    }

    pub(crate) fn validate_unscaled(self, unscaled: i128) -> Result<(), DecimalError> {
        let limit = 10_u128.pow(u32::from(self.precision));
        if unscaled.unsigned_abs() >= limit {
            return Err(DecimalError::CoefficientOutOfRange {
                decimal_type: self,
                unscaled,
            });
        }
        Ok(())
    }
}

/// One typed Decimal scalar. Arrays store only its `i128` coefficient per row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decimal {
    unscaled: i128,
    decimal_type: DecimalType,
}

pub type DecimalRef = Decimal;

impl Decimal {
    pub fn try_new(unscaled: i128, decimal_type: DecimalType) -> Result<Self, DecimalError> {
        decimal_type.validate_unscaled(unscaled)?;
        Ok(Self {
            unscaled,
            decimal_type,
        })
    }

    pub fn unscaled(self) -> i128 {
        self.unscaled
    }

    pub fn decimal_type(self) -> DecimalType {
        self.decimal_type
    }

    pub(crate) fn from_validated(unscaled: i128, decimal_type: DecimalType) -> Self {
        Self {
            unscaled,
            decimal_type,
        }
    }
}

/// A checked Decimal construction or conversion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPrecision { precision } => write!(
                formatter,
                "Decimal precision must be between 1 and {}, got {precision}",
                DecimalType::MAX_PRECISION
            ),
            Self::ScaleExceedsPrecision { precision, scale } => write!(
                formatter,
                "Decimal scale {scale} exceeds precision {precision}"
            ),
            Self::CoefficientOutOfRange {
                decimal_type,
                unscaled,
            } => write!(
                formatter,
                "unscaled Decimal coefficient {unscaled} does not fit precision {}",
                decimal_type.precision()
            ),
            Self::MetadataMismatch { expected, actual } => write!(
                formatter,
                "Decimal metadata mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::ValueValidityLength { values, validity } => write!(
                formatter,
                "Decimal value/validity length mismatch: {values} values, {validity} validity bits"
            ),
            Self::ExpectedDecimal { actual } => {
                write!(formatter, "expected a Decimal value, got {actual:?}")
            }
        }
    }
}

impl Error for DecimalError {}
