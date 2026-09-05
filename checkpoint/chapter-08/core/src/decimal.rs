use anyhow::{Result, bail};

/// Checkpoint 1's shared logical descriptor for a Decimal scalar or array.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DecimalType {
    precision: u8,
    scale: u8,
}

impl DecimalType {
    pub const MAX_PRECISION: u8 = 38;

    pub fn try_new(precision: u8, scale: u8) -> Result<Self> {
        if !(1..=Self::MAX_PRECISION).contains(&precision) {
            bail!(
                "Decimal precision must be between 1 and {}, got {precision}",
                Self::MAX_PRECISION
            );
        }
        if scale > precision {
            bail!("Decimal scale {scale} exceeds precision {precision}");
        }
        Ok(Self { precision, scale })
    }

    pub fn precision(self) -> u8 {
        self.precision
    }

    pub fn scale(self) -> u8 {
        self.scale
    }

    pub(crate) fn validate_unscaled(self, unscaled: i128) -> Result<()> {
        let limit = 10_u128.pow(u32::from(self.precision));
        if unscaled.unsigned_abs() >= limit {
            bail!(
                "unscaled Decimal coefficient {unscaled} does not fit precision {}",
                self.precision()
            );
        }
        Ok(())
    }
}

/// One typed Decimal scalar; dense arrays store only its `i128` coefficient per row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decimal {
    unscaled: i128,
    decimal_type: DecimalType,
}

pub type DecimalRef = Decimal;

impl Decimal {
    pub fn try_new(unscaled: i128, decimal_type: DecimalType) -> Result<Self> {
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
