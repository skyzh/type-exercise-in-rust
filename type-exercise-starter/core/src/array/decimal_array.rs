//! Chapter 1: define Decimal array storage and its rollback-safe builder here.
//!
//! Wrap `PrimitiveArray<i128>` so Decimal reuses its coefficient and validity storage while one
//! shared `DecimalType` remains attached to the array. Validate raw-part lengths and coefficient
//! ranges, preserve metadata across reads, and reject mixed-metadata rows before mutating the
//! builder.
