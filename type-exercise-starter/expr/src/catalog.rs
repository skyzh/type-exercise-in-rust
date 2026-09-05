//! Checkpoint 8: build the physical expression catalog here.
//!
//! Define a public `PhysicalFunction` identifier, discoverable `PhysicalFunctionEntry` metadata,
//! `PHYSICAL_FUNCTION_CATALOG`, `find_physical_function`, and `build_physical_expression`.
//! Construction accepts one identifier plus an exact `&[PhysicalType]` signature and returns
//! `anyhow::Result<Box<dyn Expression>>`.
//!
//! Catalog every maintained numeric, Boolean, and String physical function. Numeric signatures use
//! only lossless widening; clamp promotes the first pair and then its third input. Reject unknown
//! names and unsupported arities or physical signatures before evaluation. This module owns
//! selection, while the sibling scalar modules own meaning and core owns every row loop.
