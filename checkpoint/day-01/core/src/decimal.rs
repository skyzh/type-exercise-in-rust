//! Day 2, checkpoint 4: define Decimal metadata and scalar values here.
//!
//! `DecimalType` validates precision/scale once per column. `Decimal` pairs one transient i128
//! coefficient with that metadata. Use `anyhow::Result` for the few checked construction and
//! conversion boundaries; the lesson does not need a separate Decimal error taxonomy.
