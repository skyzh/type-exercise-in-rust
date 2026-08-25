//! Day 2, checkpoint 4: define Decimal array storage and its rollback-safe builder here.
//!
//! Store shared `DecimalType`, flat i128 coefficients, and packed validity. Validate raw-part
//! lengths and coefficient ranges, preserve metadata across reads, and reject mixed-metadata rows
//! before mutating the builder.
