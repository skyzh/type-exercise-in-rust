//! Learner-owned column-view checkpoints.
//!
//! Chapter 2: uncomment and implement the borrowed representations here.
// pub struct ColumnViewImpl<'a> { /* public wrapper around private representation state */ }
// impl<'a> ColumnViewImpl<'a> {
//     pub fn array(array: &'a ArrayImpl) -> Self;
//     pub fn constant(value: ScalarRefImpl<'a>, len: usize) -> Self;
//     pub fn null(physical_type: PhysicalType, len: usize) -> Self;
//     pub fn indexed(
//         indices: &'a [u32],
//         values: &'a ArrayImpl,
//     ) -> anyhow::Result<Self>;
//     pub fn len(&self) -> usize;
//     pub fn is_empty(&self) -> bool;
//     pub fn physical_type(&self) -> PhysicalType;
//     pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'a>>;
// }
//
//! Chapter 6: add the crate-private `RawI32Column` observation. Bind Int32 arrays to
//! their values and validity buffers, value/typed-null constants to one value plus a validity bit,
//! and detect Indexed views separately. Do not expose a public raw-column API.
//
//! Chapter 2: uncomment and implement the typed view here.
// pub struct ColumnView<'a, S: Scalar> { /* checked borrowed state */ }
// impl<'a, S: Scalar> ColumnView<'a, S> {
//     pub fn get(&self, row: usize) -> Option<S::RefType<'a>>;
//     pub fn len(&self) -> usize;
// }
// impl<'a, S> TryFrom<ColumnViewImpl<'a>> for ColumnView<'a, S> { /* checked conversion */ }
//
//! Chapter 9: extend this file with checked one-level List views.
// pub struct ListColumnView<'a> { /* checked List state */ }
// impl<'a> ListColumnView<'a> {
//     pub fn len(&self) -> usize;
//     pub fn get(&self, row: usize) -> Result<Option<ListScalarRef<'a>>, ListError>;
// }
// impl<'a> ColumnViewImpl<'a> {
//     pub fn try_as_list(
//         self,
//         element_type: PhysicalType,
//     ) -> Result<ListColumnView<'a>, ListError>;
// }
