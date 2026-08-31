//! Learner-owned core expression checkpoints.
//!
//! This file is compiled by the nested `type-exercise-starter-core` package. It owns every row
//! traversal. Facade operation modules supply scalar callbacks and never write batch loops.
//!
//! Day 4: define `BinaryScalarFunction`, `evaluate_binary`, and the first fixed-width
//! `BinaryExpression`. Validate arity, physical types, and lengths before evaluating rows.
//!
//! Day 5: generalize the binary shell so the facade can select one typed arithmetic or comparison
//! callback for a complete batch.
//!
//! Day 6, checkpoint 1: publish `validate_expression_inputs` from the core package.
//! Checkpoint 2: implement the shared strict unary, binary, and ternary evaluators. A null input
//! row produces a null output without calling the scalar callback.
//! Checkpoint 3: keep those three loops generic while facade arithmetic supplies only scalar
//! functions such as `neg_number` and `clamp_number`.
//!
//! Day 7: add `PrimitiveLoop` and `PrimitiveBinaryExpression`. Select raw Int32 arrays/constants
//! once, compute values in four strict-total fixed-width loops, and combine validity by storage
//! word. Indexed inputs use the general gather loop and report `PrimitiveLoop::Indexed`.
//!
//! Day 8: add nullable-aware unary/binary evaluators for SQL Boolean AND/OR. Registration chooses
//! strict versus nullable-aware evaluation before entering a row loop; there is no null-policy
//! enum and no operator match in the loop.
//!
//! Day 9: add the object-safe `Expression: Any + Send + Sync` boundary, fixed-width erased
//! adapters, and the generic registry. The builtin catalog is registered by the facade.
//!
//! Day 10: add writer-based unary/binary/ternary evaluators. A variable-width callback consumes
//! `Writer<'a>` and returns `WriterUsed<'a>`, proving exactly one non-null row is committed.
//!
//! Day 13: add checked `Any` recovery and lifetime-shortening helpers.
//!
//! Day 14: add `evaluate_static`, `BatchFuture`, `AsyncExpression`, and the erased async
//! adapter while preserving the same borrowed input lifetime and synchronous result.
