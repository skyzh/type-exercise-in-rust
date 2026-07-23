# Build a Database Expression Framework in Rust

This course uses Rust's type system to build a practical vectorized database expression framework.
It starts with a scalar (Volcano-style) evaluator, introduces Arrow-like arrays and borrowed scalar
values, and ends with planning-time type binding, runtime type erasure, generated numeric kernels,
custom data-type-specific expressions, and zero-copy column views.

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
- [Part I: Start with a Scalar Evaluator](https://skyzh.github.io/type-exercise-in-rust/volcano/overview.html)
- [Part II: Build the Vectorized Runtime](https://skyzh.github.io/type-exercise-in-rust/vectorized/overview.html)
- [Benchmarks and Next Steps](https://skyzh.github.io/type-exercise-in-rust/benchmarks.html)

To preview the book locally:

```console
mdbook serve tutorial --open
```

The course is organized around building a database component rather than cataloging isolated Rust
techniques. The repository uses stable Rust, Edition 2024, and direct higher-ranked bounds.

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
mdbook build tutorial
```

Run the generated-kernel versus hand-written-loop benchmarks:

```console
cargo bench -p expr-impl --bench expression
```

## Community

Join skyzh's Discord server to discuss the course.

[![Join skyzh's Discord Server](tutorial/src/discord-badge.svg)](https://skyzh.dev/join/discord)

## License

The source code is licensed under Apache 2.0. See [LICENSE](./LICENSE). The mdBook text is
© 2022-2026 Alex Chi Z and licensed under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).
