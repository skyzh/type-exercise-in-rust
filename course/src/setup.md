{{#include wip-banner.md}}

# Environment Setup

Install [rustup](https://rustup.rs/), update stable Rust, and install
[mdBook](https://rust-lang.github.io/mdBook/):

```console
rustup update stable
rustc --version
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

- Work only in implementation files under `type-exercise-starter/src/`.
- Do not edit `src/tests.rs` or copied files under `src/tests/`.
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

Chapter commands target the facade package, `type-exercise-starter-expr`. When a chapter asks you
to verify that reusable machinery is independent of concrete operations, it additionally runs
`type-exercise-starter-core`. Both packages use the same starter source directory; the Cargo edge
is always facade to core.

## Preview the course

```console
mdbook serve course --open
```

Continue to [Chapter 1: Connect One Type Family by Hand](./chapter-1-type-family.md).

{{#include copyright.md}}
