//! Learner-owned SQL Boolean scalar operations.
//!
//! Chapter 6 establishes the nullable-aware core adapter with a supplied callback; it does not
//! enable this concrete facade.
//!
//! Chapter 8: implement NOT, AND, and OR scalar semantics, then add crate-private physical
//! factories for the binder. NOT is strict; AND and OR inspect `Option<bool>` so
//! `FALSE AND NULL` is false and `TRUE OR NULL` is true. Keep the exhaustive truth rows in
//! supplied tests, add no null-policy enum, and keep operation selection outside the row loop.
//! Boolean equality selection belongs here too; `numeric.rs` owns numeric operations only.
