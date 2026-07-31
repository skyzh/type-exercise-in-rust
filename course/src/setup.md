# Environment Setup

The workspace uses the stable Rust toolchain and Edition 2024. Install
[rustup](https://rustup.rs/) and [mdBook](https://rust-lang.github.io/mdBook/), then run these
commands from the repository root:

```console
rustc --version
cargo test --workspace --all-targets --locked
mdbook build course
```

The first command should report the currently installed stable Rust release. The test command
compiles every workspace crate and runs the supplied tests. The book is written to `course/book/`.

## Starter Map

| Path | Supplied responsibility |
| --- | --- |
| `expr-common` | Arrays, array builders, owned and borrowed scalars, erased value enums, and expression interfaces |
| `expr-template-impl` | Rust code generation for expression arities one through five |
| `expr-template` | Generated typed evaluators and the runtime adapter |
| `expr-macro-rules` | Logical-to-physical type association macros |
| `expr-impl` | Concrete kernels and the runtime expression constructor |
| `archive` | Historical snapshots from the original course; useful context, but not the active implementation |

Generated files under `expr-template/src/gen/` are build outputs. Change the generator in
`expr-template-impl/src/lib.rs`; a normal Cargo build regenerates the files.

## Daily Validation

Run the focused command at the end of each chapter while you work. Before you finish a day, run:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
mdbook test course
```

Do not change public interfaces, generated output, or unrelated tests merely to make a check pass.
The chapter contract names the files that belong to each day and the behavior that must remain
stable.

Continue to [Day 1](./day-1-column-views.md).

{{#include copyright.md}}
