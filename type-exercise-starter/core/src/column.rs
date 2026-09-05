//! Checkpoint 2: add borrowed nullable column views here.
//!
//! `ColumnViewImpl<'a>` keeps one private representation with three public constructors:
//! an `Array` view over `&'a ArrayImpl`, a repeated `Constant` (including a typed null), and an
//! `Indexed` view over borrowed row indices plus an existing array. Validate every index during
//! construction, then expose `len`, `is_empty`, `physical_type`, and `get` without materializing
//! another array.
//!
//! Add `ColumnView<'a, S: Scalar>` as the checked typed form. Its `TryFrom<ColumnViewImpl<'a>>`
//! implementation checks the physical family once; `get` then returns `Option<S::RefType<'a>>`.
//! Keep both representation enums private so callers cannot bypass constructor and type checks.
//!
//! The required public surface is:
//!
//! ```rust,ignore
//! pub struct ColumnViewImpl<'a> { /* private representation */ }
//! impl<'a> ColumnViewImpl<'a> {
//!     pub fn array(array: &'a ArrayImpl) -> Self;
//!     pub fn constant(value: ScalarRefImpl<'a>, len: usize) -> Self;
//!     pub fn null(physical_type: PhysicalType, len: usize) -> Self;
//!     pub fn indexed(indices: &'a [u32], values: &'a ArrayImpl) -> anyhow::Result<Self>;
//!     pub fn len(&self) -> usize;
//!     pub fn is_empty(&self) -> bool;
//!     pub fn physical_type(&self) -> PhysicalType;
//!     pub fn get(&self, row: usize) -> Option<ScalarRefImpl<'a>>;
//! }
//!
//! pub struct ColumnView<'a, S: Scalar> { /* private checked representation */ }
//! impl<'a, S: Scalar> ColumnView<'a, S> {
//!     pub fn len(&self) -> usize;
//!     pub fn is_empty(&self) -> bool;
//!     pub fn get(&self, row: usize) -> Option<S::RefType<'a>>;
//! }
//! ```
//!
//! Checkpoint 5: let the core expression module inspect the typed Array/Constant/Indexed kind
//! without making that representation public. Callers must continue using the checked view API.
//!
//! Raw-buffer observations and List views belong to later checkpoints.
