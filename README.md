# Build a Database Expression Framework in Rust

This course uses Rust's type system to build a practical vectorized database expression framework.
It starts from supplied Arrow-like arrays, borrowed scalars, and generated expression templates,
then adds zero-copy column views, planning-time binding, specialized primitive loops, stronger Rust
type boundaries, and a batch-level asynchronous adapter.

The central design rule is deliberately narrow:

- Use generic expansion for expression families with a real cross-product, especially numeric
  promotion and comparison.
- Implement most other database expressions as explicit kernels for the data types they understand.
- Keep the common framework easy to extend without forcing every expression through the same type
  exercise.

`ColumnView` lets a kernel read a logical column backed by a regular Arrow-like array, a dictionary
encoding, or a repeated constant. The expression does not materialize or special-case those inputs.

## Book

Read the published mdBook at
[Build a Database Expression Framework in Rust](https://skyzh.github.io/type-exercise-in-rust/):

- [Preface](https://skyzh.github.io/type-exercise-in-rust/preface.html)
- [Day 1: Read Arrays, Constants, and Dictionaries](https://skyzh.github.io/type-exercise-in-rust/day-1-column-views.html)

To preview the book locally:

```console
mdbook serve course --open
```

The five-day progression is column views, expression binding, fast paths and benchmarks, modern
Rust type boundaries, and asynchronous batch boundaries. Each day adds the implementation and the
chapter that explains its contract.

## Workspace

- `expr-common`: arrays, scalars, logical types, type erasure, and column views.
- `expr-template-impl`: code generator for expression arities one through five.
- `expr-template`: generated vectorized expression types.
- `expr-macro-rules`: logical-to-physical type association macros.
- `expr-impl`: numeric/comparison families, custom kernels, binder, and function registry.
- `archive`: historical day-by-day snapshots from the original course.

Run the complete validation suite:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
mdbook test course
```

## Community

Join [skyzh's Discord server](https://skyzh.dev/join/discord) to discuss the course.

## License

The source code is licensed under Apache 2.0. See [LICENSE](./LICENSE). The mdBook text is
© 2022-2026 Alex Chi Z and licensed under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).
