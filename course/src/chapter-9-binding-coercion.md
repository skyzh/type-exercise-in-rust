# Chapter 9: Erase Typed Expressions at Runtime

The evaluator families are generic so Rust can specialize their scalar and row work. A query plan,
however, needs one collection containing expressions with different concrete types and arities.
Erasure belongs around the complete batch expression—not around each scalar operation.

This chapter introduces that boundary in three stages. The typed evaluator remains responsible for
validation, nulls, row errors, and output construction. The erased shell exposes stable metadata
and delegates one whole batch.

## Checkpoint 1: one object-safe expression boundary

```console
cargo x copy-test --chapter 9 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_9 --locked
```

Define the object-safe `Expression: Any + Send + Sync` trait with name, input physical types,
output type, and `evaluate`. Store the complete input signature as a slice; deriving
arity from `input_types().len()` keeps unary, binary, and ternary metadata consistent.

Chapter 8 already leaves evaluation-only `Expression` implementations on the existing typed shells.
Publish truthful metadata for each of those implementations when you extend the trait so the
predecessor still compiles. Adapt only `i32_add` to `Box<dyn Expression>` at this checkpoint; the
other erased families arrive next. The two focused tests pass here, for 63 cumulative tests.
Do not place `dyn Fn` inside the row loop.

## Checkpoint 2: preserve typed validation through erasure

```console
cargo x copy-test --chapter 9 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_9 --locked
```

Add erased adapters for the complete typed evaluator families. Their job is deliberately narrow:

1. publish name and physical metadata;
2. pass borrowed erased column views to the selected typed kernel; and
3. verify that the returned physical family matches the declared output.

Do not repeat arity, type, length, strict-null, or row-context logic in the adapter. The ten
focused tests cover unary, binary, and ternary shells while preserving errors and nulls at the
typed boundary, including a deliberately wrong kernel result. This checkpoint
reaches 71 cumulative tests without requiring the catalog or Boolean erasure from checkpoint 3.

## Checkpoint 3: assemble the fixed builtin catalog

```console
cargo x copy-test --chapter 9 --checkpoint 3
cargo test -p type-exercise-starter-supplied-tests chapter_9 --locked
cargo check -p type-exercise-starter-core --locked
```

Build the finite builtin catalog, add Boolean delegation, and confirm safe `Send + Sync` sharing for
the course. Each catalog entry stores one preselected batch kernel. It may describe arithmetic,
comparison, or Boolean work, but it does not inspect an operation for every row.

The 13 focused tests cover:

- complete builtin coverage and metadata;
- typed validation and strict-null behavior through trait objects;
- unary, binary, and ternary delegation;
- Boolean delegation through the same erased surface; and
- safe sharing of erased expressions across threads.

The core-only command is the architectural control: storage, views, generic evaluators, and
erasure compile without importing the facade's concrete function catalog.

## Why erasure comes after specialization

Erasing `i32`, `f64`, or the scalar operation in every row would replace compile-time selection
with repeated runtime matching. Erasing the whole `Expression` lets a planner keep heterogeneous
objects while each object still owns a monomorphized batch kernel. Runtime flexibility and typed
inner loops are complementary when the boundary is placed at batch granularity.

The runtime boundary is now complete. Chapter 10 uses it for a result whose physical storage must
be published transactionally: [variable-width strings](./chapter-10-primitive-loops.md).

{{#include copyright.md}}
