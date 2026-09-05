//! Checkpoint 8: implement SQL Boolean scalar operations and concrete whole-batch builders here.
//!
//! NOT is strict. AND and OR receive `Option<bool>` so `FALSE AND NULL` is false and
//! `TRUE OR NULL` is true. Build fixed-arity expressions by selecting the existing strict or
//! nullable-aware core evaluator once per batch. Boolean equality also belongs here; keep all row
//! traversal in core.
