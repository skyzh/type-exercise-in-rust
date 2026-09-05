//! Checkpoint 9: bind logical calls to the completed physical catalog here.
//!
//! Add `LogicalCall`, `BindError`, `BoundExpression`, and `bind_logical_call`. Resolve only the
//! maintained logical names and lossless numeric or Char/Varchar compatibility. Select one
//! existing `PhysicalFunction`, build it once, verify its physical metadata, and delegate the
//! complete batch through `Box<dyn Expression>`.
