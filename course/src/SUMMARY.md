# Summary

[Preface](./preface.md)
[Environment Setup](./setup.md)

# Physical values and lazy columns

- [Build Physical Types and Arrays](./chapter-1-type-family.md)
- [Read Nullable Columns Lazily](./chapter-2-column-views.md)

# Shared evaluation

- [Build Shared Typed Evaluation](./chapter-3-shared-evaluation.md)
- [Build Variable-Width Rows Transactionally](./chapter-4-concrete-loops.md)

# Specialized evaluation

- [Specialize Common Column Shapes](./chapter-5-generic-arithmetic.md)
- [Separate Fast Paths from Semantic Exceptions](./chapter-6-systematic-arity.md)

# Runtime planning

- [Erase One Complete Batch](./chapter-7-boolean-logic.md)
- [Bind Logical Calls Once](./chapter-8-runtime-erasure.md)

# Nested storage and Rust boundaries

- [Build a One-Level List Column](./chapter-9-binding-coercion.md)
- [Share and Schedule a Batch Safely](./chapter-10-primitive-loops.md)
