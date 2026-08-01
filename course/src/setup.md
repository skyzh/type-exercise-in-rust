# Environment Setup

Install [rustup](https://rustup.rs/), update to the latest stable Rust, and install
[mdBook](https://rust-lang.github.io/mdBook/):

```console
rustup update stable
rustc --version
```

The repository's `rust-toolchain` selects the rolling `stable` channel, and the learner crate uses
Rust Edition 2024. It does not declare an older minimum supported Rust version. The learner
workspace is the standalone crate in `type-exercise-starter/`; it is intentionally separate from
the completed historical implementations under `archived/`. The repository root is a course
container, not a Cargo workspace.

## Check Out the Starting State

Clone the repository, then create your working branch from the durable starter branch:

```console
git fetch origin
git switch --create course-work --track origin/skyzh/course-starter
```

If `course-work` already exists, choose another local branch name. Confirm that the starter contains
only `ScalarImpl`, its baseline test, and the gated chapter contract tests:

```console
find type-exercise-starter -maxdepth 3 -type f | sort
cargo check --manifest-path type-exercise-starter/Cargo.toml --lib --locked
cargo test --manifest-path type-exercise-starter/Cargo.toml --locked
```

The check and test commands should succeed. Chapter contract tests are behind Cargo features, so
they become executable specifications only when you run the matching chapter command. Before you
implement that chapter, the featured test is expected to report missing APIs.

## Learner Boundary

For the first two chapters:

- modify only `type-exercise-starter/src/`;
- keep `type-exercise-starter/tests/`, public names, feature names, and dependencies unchanged;
- do not add macros, unsafe code, expression traits, generated code, or additional scalar types;
- use the current chapter, `type-exercise-starter/README.md`, supplied tests, compiler diagnostics, and official
  Rust documentation as implementation sources; and
- leave `archived/` unread until after your chapter reflection; it contains broader historical
  implementations, not exercise inputs or dependencies.

There is intentionally no Cargo workspace at the repository root. Always use the exact
`--manifest-path type-exercise-starter/Cargo.toml` command from the chapter.

## Preview the Course

Run this from the repository root:

```console
mdbook serve course --open
```

The generated book is written to `course/book/` and is not committed.

Continue to [Chapter 1: Connect Scalars, References, and Arrays](./chapter-1-type-connections.md).

{{#include copyright.md}}
