//! Day 2, checkpoint 4: define Decimal metadata, scalar values, and readable errors here.
//!
//! `DecimalType` validates precision/scale once per column. `Decimal` pairs one transient i128
//! coefficient with that metadata. `DecimalError` covers invalid metadata, coefficients, array
//! shape, metadata mismatches, and erased-type mismatches.
