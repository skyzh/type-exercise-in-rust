# Summary

[Preface](./preface.md)
[Environment Setup](./setup.md)

# Type families

- [Connect One Type Family by Hand](./chapter-1-type-family.md)
- [Scale the Physical Type Family](./chapter-2-type-catalog.md)

# Borrowed columns and first batch evaluation

- [Read Nullable Columns Without Materializing Them](./chapter-3-column-views.md)
- [Expose the Cost of Concrete Loops](./chapter-4-concrete-loops.md)

# Generic numeric evaluation

- [Make Numeric Evaluation Generic](./chapter-5-generic-arithmetic.md)
- [Make Arity Systematic](./chapter-6-systematic-arity.md)

# Specialized execution and Boolean nulls

- [Select Dense Fixed-Width Loops](./chapter-7-boolean-logic.md)
- [Implement Three-Valued Boolean Logic](./chapter-8-runtime-erasure.md)

# Runtime expressions and variable-width output

- [Erase Typed Expressions at Runtime](./chapter-9-binding-coercion.md)
- [Build Variable-Width Strings Transactionally](./chapter-10-primitive-loops.md)

# Logical binding and nested storage

- [Bind and Coerce Logical Calls](./chapter-11-list.md)
- [Build a One-Level List Column](./chapter-12-rust-boundaries.md)

# Thread-safe and async boundaries

- [Share Logical Factories Across Threads](./chapter-13-async-boundary.md)
- [Add a Batch Async Boundary](./chapter-14-async-boundary.md)
