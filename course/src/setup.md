# Environment Setup

Install [rustup](https://rustup.rs/), update stable Rust, and install
[mdBook](https://rust-lang.github.io/mdBook/) plus `cargo-expand`:

```console
rustup update stable
rustc --version
cargo install cargo-expand --locked
cargo expand --version
```

The repository selects rolling `stable` and Rust Edition 2024. It does not claim an older minimum
supported Rust version.

## Check out the starting state

Clone the repository, then create a working branch from `main`:

```console
git fetch origin
git switch --create course-work --track origin/main
```

Choose another branch name if `course-work` already exists. Verify the untouched starter:

```console
cargo check -p type-exercise-starter-core --locked
cargo test -p type-exercise-starter-expr --lib --locked
```

Both commands should pass. Chapter tests do not exist in the starter until you copy them.

## Follow the learner boundary

- Work only in implementation files under `type-exercise-starter/expr/src/` and
  `type-exercise-starter/core/src/`.
- Do not edit `supplied-tests/src/lib.rs` or copied files under `supplied-tests/src/`.
- Do not read, search, diff, or copy `type-exercise/`, `archived/`, Git history, or an online
  solution while implementing a chapter.
- Use the chapter, copied destination test, compiler diagnostics, and official Rust documentation.
- Add only the types, modules, dependencies, and public APIs owned by the current chapter.

The only permitted reference-to-starter operation is:

```console
cargo x copy-test --chapter <N>
```

Run it without opening the source test. The command copies the cumulative tests through chapter
`N`, removes later managed tests, and regenerates the module list. Afterward, you may read the
copied destination. Its first focused run should be red until you implement the chapter.

Focused chapter commands target `type-exercise-starter-supplied-tests`. For an implementation-only
check, target either the facade package, `type-exercise-starter-expr`, or the reusable framework
package, `type-exercise-starter-core`. Their sources live separately under `expr/src/` and
`core/src/`. Dependencies point from supplied tests to the facade to core, with a direct
supplied-tests-to-core edge for framework witnesses.

## Preview the course

```console
mdbook serve course --open
```

Continue to [Checkpoint 1: Build Physical Types and Arrays](./chapter-1-type-family.md).

{{#include copyright.md}}
