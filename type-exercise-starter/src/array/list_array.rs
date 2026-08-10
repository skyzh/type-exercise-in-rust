//! Learner-owned one-level List checkpoints.
//!
//! Day 11, checkpoint 1: add typed owned and borrowed List scalars here.
// pub enum ListError { /* invalid shape, type, range, and nesting failures */ }
// pub struct ListScalar { /* owned child array */ }
// pub struct ListScalarRef<'a> { /* borrowed child range */ }
//
//! Day 11, checkpoint 2: add checked outer List storage here.
// pub struct ListArray { /* child type, child values, offsets, and outer validity */ }
// pub(crate) struct ListArrayBuilder { /* rollback-safe construction state */ }
// impl ListArray {
//     pub fn try_from_rows(/* child type and rows */) -> Result<Self, ListError>;
//     pub fn try_from_raw_parts(/* child values, offsets, and validity */) -> Result<Self, ListError>;
// }
