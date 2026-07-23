# Environment Setup

The repository pins Rust 1.94 and uses Rust Edition 2024. Edition 2024 requires Rust 1.85 or newer,
but using the pinned toolchain makes generated code and diagnostics reproducible.

Install [rustup](https://rustup.rs/) and mdBook, then verify the workspace:

```console
rustc --version
cargo test --workspace --all-targets
mdbook build tutorial
```

To read the book with live reload:

```console
mdbook serve tutorial --open
```

## Workspace Map

The final implementation is split by responsibility:

| Crate | Responsibility |
| --- | --- |
| `expr-common` | Physical arrays, scalar traits, logical types, column views, and erased enums |
| `expr-template-impl` | Rust code generator for expression arities one through five |
| `expr-template` | Generated typed expression structs and the runtime adapter |
| `expr-macro-rules` | Logical-to-physical type association macros |
| `expr-impl` | Kernels, promotion matrices, binder, registry, tests, and benchmarks |

The `archive` directory contains historical checkpoints from the original course. They are useful
for seeing how the design evolved, but they intentionally preserve older APIs and explanations. Use
the workspace crates as the source of truth for this book.

## Validation Commands

Run these before completing a chapter that changes code:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```

The generated files in `expr-template/src/gen` are written by `expr-template/build.rs`. Edit
`expr-template-impl/src/lib.rs`, not the generated output. A normal Cargo build regenerates them.

## Learning Strategy

The repository already contains the reference implementation. For a hands-on course, create a
branch and replace selected methods with `todo!()`, or use the chapter tasks as prompts for a fresh
crate. Keep the public type signatures visible: the exercise is to make the relationships compile,
not to guess the intended API from scratch.

You are ready to begin with [a scalar evaluator](./volcano/overview.md).

{{#include copyright.md}}
