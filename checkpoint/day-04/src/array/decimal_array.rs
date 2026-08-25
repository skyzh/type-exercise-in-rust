use bitvec::vec::BitVec;

use crate::{Decimal, DecimalError, DecimalType};

/// A nullable Decimal array with one descriptor shared by all rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecimalArray {
    values: Vec<i128>,
    validity: BitVec,
    decimal_type: DecimalType,
    null_count: usize,
}

impl DecimalArray {
    pub fn try_from_raw_parts(
        decimal_type: DecimalType,
        values: Vec<i128>,
        validity: BitVec,
    ) -> Result<Self, DecimalError> {
        if values.len() != validity.len() {
            return Err(DecimalError::ValueValidityLength {
                values: values.len(),
                validity: validity.len(),
            });
        }
        for (value, valid) in values.iter().copied().zip(validity.iter().by_vals()) {
            if valid {
                decimal_type.validate_unscaled(value)?;
            }
        }
        let null_count = validity.iter().by_vals().filter(|valid| !valid).count();
        Ok(Self {
            values,
            validity,
            decimal_type,
            null_count,
        })
    }

    pub fn try_from_slice(
        decimal_type: DecimalType,
        values: &[Option<Decimal>],
    ) -> Result<Self, DecimalError> {
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
        &self.values
    }

    pub fn validity(&self) -> &BitVec {
        &self.validity
    }

    pub fn null_count(&self) -> usize {
        self.null_count
    }

    pub fn get(&self, row: usize) -> Option<Decimal> {
        if row >= self.len() || !self.validity[row] {
            return None;
        }
        Some(Decimal::from_validated(self.values[row], self.decimal_type))
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    // Day 11 adds checked slicing for List child storage.
}

/// A fail-closed Decimal builder that requires metadata before any row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecimalArrayBuilder {
    values: Vec<i128>,
    validity: BitVec,
    decimal_type: DecimalType,
    null_count: usize,
}

impl DecimalArrayBuilder {
    pub fn try_with_type(decimal_type: DecimalType, capacity: usize) -> Result<Self, DecimalError> {
        // `DecimalType` can only be created by its checked constructor. Keep this
        // result fallible so metadata-bearing builders share one explicit boundary.
        Ok(Self {
            values: Vec::with_capacity(capacity),
            validity: BitVec::with_capacity(capacity),
            decimal_type,
            null_count: 0,
        })
    }

    pub fn try_push(&mut self, value: Option<Decimal>) -> Result<(), DecimalError> {
        if let Some(value) = value {
            if value.decimal_type() != self.decimal_type {
                return Err(DecimalError::MetadataMismatch {
                    expected: self.decimal_type,
                    actual: value.decimal_type(),
                });
            }
            self.decimal_type.validate_unscaled(value.unscaled())?;
            self.values.push(value.unscaled());
            self.validity.push(true);
        } else {
            self.values.push(0);
            self.validity.push(false);
            self.null_count += 1;
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn finish(self) -> DecimalArray {
        DecimalArray {
            values: self.values,
            validity: self.validity,
            decimal_type: self.decimal_type,
            null_count: self.null_count,
        }
    }
}
