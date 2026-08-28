use anyhow::{Result, bail};
use bitvec::vec::BitVec;

use crate::{Decimal, DecimalType, PrimitiveArray, PrimitiveArrayBuilder};

/// A nullable Decimal array with one descriptor shared by all rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecimalArray {
    storage: PrimitiveArray<i128>,
    decimal_type: DecimalType,
}

impl DecimalArray {
    pub fn try_from_raw_parts(
        decimal_type: DecimalType,
        values: Vec<i128>,
        validity: BitVec,
    ) -> Result<Self> {
        if values.len() != validity.len() {
            bail!(
                "Decimal value/validity length mismatch: {} values, {} validity bits",
                values.len(),
                validity.len()
            );
        }
        for (value, valid) in values.iter().copied().zip(validity.iter().by_vals()) {
            if valid {
                decimal_type.validate_unscaled(value)?;
            }
        }
        Ok(Self {
            storage: PrimitiveArray::from_raw_parts(values, validity),
            decimal_type,
        })
    }

    pub fn try_from_slice(decimal_type: DecimalType, values: &[Option<Decimal>]) -> Result<Self> {
        let mut builder = DecimalArrayBuilder::try_with_type(decimal_type, values.len())?;
        for value in values {
            builder.try_push(*value)?;
        }
        Ok(builder.finish())
    }

    pub fn decimal_type(&self) -> DecimalType {
        self.decimal_type
    }

    pub fn values(&self) -> &[i128] {
        self.storage.values()
    }

    pub fn validity(&self) -> &BitVec {
        self.storage.validity()
    }

    pub fn get(&self, row: usize) -> Option<Decimal> {
        if row >= self.len() || !self.storage.validity()[row] {
            return None;
        }
        Some(Decimal::from_validated(
            self.storage.values()[row],
            self.decimal_type,
        ))
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.storage.values().is_empty()
    }

    // Day 11 adds checked slicing for List child storage.
}

/// A fail-closed Decimal builder that requires metadata before any row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecimalArrayBuilder {
    storage: PrimitiveArrayBuilder<i128>,
    decimal_type: DecimalType,
}

impl DecimalArrayBuilder {
    pub fn try_with_type(decimal_type: DecimalType, capacity: usize) -> Result<Self> {
        // `DecimalType` can only be created by its checked constructor. Keep this
        // result fallible so metadata-bearing builders share one explicit boundary.
        Ok(Self {
            storage: PrimitiveArrayBuilder::with_raw_capacity(capacity),
            decimal_type,
        })
    }

    pub fn try_push(&mut self, value: Option<Decimal>) -> Result<()> {
        if let Some(value) = value {
            if value.decimal_type() != self.decimal_type {
                bail!(
                    "Decimal metadata mismatch: expected {:?}, got {:?}",
                    self.decimal_type,
                    value.decimal_type()
                );
            }
            self.decimal_type.validate_unscaled(value.unscaled())?;
            self.storage.push_raw(value.unscaled(), true);
        } else {
            self.storage.push_raw(0, false);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.storage.len() == 0
    }

    pub fn finish(self) -> DecimalArray {
        DecimalArray {
            storage: self.storage.finish_raw(),
            decimal_type: self.decimal_type,
        }
    }
}
