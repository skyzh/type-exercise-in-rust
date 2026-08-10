use bitvec::vec::BitVec;

use crate::{Decimal, DecimalError, DecimalType};

/// Day 2, checkpoint 4: implement flat `i128` coefficients, packed validity, and shared metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecimalArray;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Day 2, checkpoint 4: implement rollback-safe Decimal construction.
pub struct DecimalArrayBuilder;

impl DecimalArray {
    pub fn try_from_raw_parts(
        _: DecimalType,
        _: Vec<i128>,
        _: BitVec,
    ) -> Result<Self, DecimalError> {
        todo!("validate Decimal raw parts in Day 2")
    }
    pub fn try_from_slice(_: DecimalType, _: &[Option<Decimal>]) -> Result<Self, DecimalError> {
        todo!("build a typed Decimal array in Day 2")
    }
    pub fn decimal_type(&self) -> DecimalType {
        todo!("preserve shared Decimal metadata in Day 2")
    }
    pub fn values(&self) -> &[i128] {
        todo!("store flat Decimal coefficients in Day 2")
    }
    pub fn validity(&self) -> &BitVec {
        todo!("store packed Decimal validity in Day 2")
    }
    pub fn get(&self, _: usize) -> Option<Decimal> {
        todo!("read a typed Decimal scalar in Day 2")
    }
    pub fn null_count(&self) -> usize {
        todo!("count Decimal nulls in Day 2")
    }
    pub fn len(&self) -> usize {
        todo!("report Decimal row count in Day 2")
    }
    pub fn is_empty(&self) -> bool {
        todo!("report whether a Decimal array has no rows in Day 2")
    }
}

impl DecimalArrayBuilder {
    pub fn try_with_type(_: DecimalType, _: usize) -> Result<Self, DecimalError> {
        todo!("require Decimal metadata before rows in Day 2")
    }
    pub fn try_push(&mut self, _: Option<Decimal>) -> Result<(), DecimalError> {
        todo!("append checked Decimal rows in Day 2")
    }
    pub fn finish(self) -> DecimalArray {
        todo!("finish a Decimal array in Day 2")
    }
}
