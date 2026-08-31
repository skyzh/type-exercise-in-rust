//! Learner-owned SQL Boolean scalar operations.
//!
//! Day 8, checkpoint 1: implement the 21-row SQL truth table as three scalar functions. NOT is
//! strict; AND and OR inspect `Option<bool>` so `FALSE AND NULL` is false and
//! `TRUE OR NULL` is true.
//!
//! Checkpoint 2: build Boolean expressions by selecting AND, OR, or NOT once, then delegate to the
//! matching strict or nullable-aware core evaluator. Keep the exhaustive truth rows in supplied
//! tests. Do not add a null-policy enum or an operation match inside a row loop.
