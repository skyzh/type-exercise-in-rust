# Environment Setup

Install [rustup](https://rustup.rs/), update to the latest stable Rust, and install
[mdBook](https://rust-lang.github.io/mdBook/):

```console
rustup update stable
rustc --version
```

The repository's `rust-toolchain` selects the rolling `stable` channel, and the learner crate uses
Rust Edition 2024. It does not declare an older minimum supported Rust version. The root workspace
contains the minimal learner crate, a separate reference solution, and the `xtask` support tool.

## Check Out the Starting State

Clone the repository, then create your working branch from `main`:

```console
git fetch origin
git switch --create course-work --track origin/main
```

If `course-work` already exists, choose another local branch name. Confirm that the starter contains
only `ScalarImpl`, its baseline test, and the empty copied-test module list:

```console
find type-exercise-starter -maxdepth 3 -type f | sort
cargo check -p type-exercise-starter --lib --locked
cargo test -p type-exercise-starter --lib --locked
```

Both commands should succeed. Chapter tests are not present in the starter until you intentionally
copy one with `cargo x copy-test --chapter <N>`.

## Learner Boundary

For the first two chapters:

- modify implementation files only under `type-exercise-starter/src/`;
- keep copied files under `type-exercise-starter/src/tests/`, `src/tests.rs`, public names, and dependencies unchanged;
- do not add macros, unsafe code, expression traits, generated code, or additional scalar types;
- use the current chapter, `type-exercise-starter/README.md`, copied tests, compiler diagnostics,
  and official Rust documentation as implementation sources; and
- do not read, search, diff, or copy `type-exercise/` or `archived/` while solving a chapter.

The only allowed reference-to-starter operation is the exact repository-root test-copy command
shown in each chapter. Run it without opening the source test first; afterward you may inspect the
copied destination. Coding agents launched inside `type-exercise-starter/` receive the same rule
from its `AGENTS.md`.

## Preview the Course

Run this from the repository root:

```console
mdbook serve course --open
```

The generated book is written to `course/book/` and is not committed.

Continue to [Chapter 1: Connect Scalars, References, and Arrays](./chapter-1-type-connections.md).

{{#include copyright.md}}
