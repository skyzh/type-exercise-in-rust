//! Learner-owned one-level List checkpoints.
//!
//! Day 11, checkpoint 1: uncomment these declarations, choose useful private fields, and add the
//! checked owned/borrowed List behavior described by the chapter.
// #[derive(Clone, Debug, Eq, PartialEq)]
// pub enum ListError { /* invalid shape, type, range, and nesting failures */ }
// #[derive(Clone, Debug, PartialEq)]
// pub struct ListScalar { /* owned child array */ }
// impl ListScalar {
//     pub fn try_new(values: ArrayImpl) -> Result<Self, ListError>;
//     pub fn element_type(&self) -> PhysicalType;
//     pub fn len(&self) -> usize;
//     pub fn get(&self, index: usize) -> Result<Option<ScalarRefImpl<'_>>, ListError>;
//     pub fn as_list_ref(&self) -> ListScalarRef<'_>;
//     pub fn slice(&self, start: usize, end: usize) -> Result<Self, ListError>;
// }
// #[derive(Clone, Copy, Debug, PartialEq)]
// pub struct ListScalarRef<'a> { /* borrowed child range */ }
// impl<'a> ListScalarRef<'a> {
//     pub fn element_type(self) -> PhysicalType;
//     pub fn len(self) -> usize;
//     pub fn get(self, index: usize) -> Result<Option<ScalarRefImpl<'a>>, ListError>;
//     pub fn slice(self, start: usize, end: usize) -> Result<Self, ListError>;
//     pub fn to_owned_scalar(self) -> Result<ListScalar, ListError>;
// }
//
//! Day 11, checkpoint 2: uncomment these declarations and add checked outer List storage here.
// #[derive(Clone, Debug, PartialEq)]
// pub struct ListArray { /* child type, child values, offsets, and outer validity */ }
// pub(crate) struct ListArrayBuilder { /* rollback-safe construction state */ }
// impl ListArray {
//     pub fn try_from_rows<'a>(
//         element_type: PhysicalType,
//         rows: impl IntoIterator<Item = Option<ListScalarRef<'a>>>,
//     ) -> Result<Self, ListError>;
//     pub fn try_from_raw_parts(
//         element_type: PhysicalType,
//         values: ArrayImpl,
//         offsets: Vec<usize>,
//         validity: Vec<bool>,
//     ) -> Result<Self, ListError>;
//     pub fn element_type(&self) -> PhysicalType;
//     pub fn values(&self) -> &ArrayImpl;
//     pub fn offsets(&self) -> &[usize];
//     pub fn validity(&self) -> &[bool];
//     pub fn len(&self) -> usize;
//     pub fn get(&self, row: usize) -> Result<Option<ListScalarRef<'_>>, ListError>;
//     pub fn slice(&self, start: usize, end: usize) -> Result<Self, ListError>;
// }
// impl ListArrayBuilder {
//     pub(crate) fn new(element_type: PhysicalType, capacity: usize) -> Result<Self, ListError>;
//     pub(crate) fn push(&mut self, value: Option<ListScalarRef<'_>>) -> Result<(), ListError>;
//     pub(crate) fn finish(self) -> Result<ListArray, ListError>;
// }
