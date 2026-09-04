//! Learner-owned SQL Boolean scalar operations.
//!
//! Chapter 6: implement the 21-row SQL truth table as three scalar functions, then expose
//! one public Boolean expression that selects the strict or nullable-aware core evaluator. NOT is
//! strict; AND and OR inspect `Option<bool>` so `FALSE AND NULL` is false and `TRUE OR NULL` is
//! true.
//!
//! Then add the public operator/arity/type metadata and exercise the same evaluator with
//! arrays and invalid batches. Keep the exhaustive truth rows in supplied tests. Do not add a
//! null-policy enum or an operation match inside a row loop.
//! Boolean equality selection also belongs here once logical binding arrives; `numeric.rs` owns
//! numeric operations only.
