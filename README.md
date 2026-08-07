# Build a Database Expression Framework in Rust

This course builds a small vectorized database expression framework from an explicit Rust starter.
The starter contains one owned scalar enum and no trait relationships. A separate reference crate
contains the maintained solution and canonical tests; an `xtask` copies each selected test into
the starter when the learner is ready to validate a chapter.

The current review boundary contains six chapters:

1. connect `i32`, `String`, their borrowed forms, and their physical arrays with traits and generic
   associated types;
2. read arrays, repeated constants, and dictionary-encoded values through one borrowed interface;
3. vectorize a typed scalar function over nullable inputs;
4. erase typed expressions behind runtime metadata and generate a builtin catalog;
5. bind logical signatures to physical expressions; and
6. select and measure all-valid primitive loops while preserving every general fallback.

Later chapters will strengthen Rust type boundaries and add a batch-level async adapter. These
topics are not forced into a fixed calendar.

## Start Here

The learner workspace is [`type-exercise-starter`](./type-exercise-starter). Begin from `main`,
where its only implemented data model is `ScalarImpl::{Int32, String}`:

```console
git fetch origin
git switch --create course-work --track origin/main
cargo check -p type-exercise-starter --lib --locked
mdbook serve course --open
```

The repository workspace contains the learner crate, [`type-exercise`](./type-exercise) reference
solution, and [`xtask`](./xtask) support tool. Learners must not inspect the reference or anything
under [`archived/`](./archived) while implementing a chapter. The starter-local `AGENTS.md`
enforces that boundary for coding agents.

Read the mdBook source in [`course`](./course), beginning with the
[preface](./course/src/preface.md), [setup](./course/src/setup.md), and
[Chapter 1](./course/src/chapter-1-type-connections.md). Continue with
[Chapter 2](./course/src/chapter-2-column-views.md),
[Chapter 3](./course/src/chapter-3-vectorize-scalar.md),
[Chapter 4](./course/src/chapter-4-expression-erasure.md),
[Chapter 5](./course/src/chapter-5-logical-binding.md), and
[Chapter 6](./course/src/chapter-6-primitive-loops.md).

Copy a chapter's canonical test into the starter with the same workflow used by Mini-LSM:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter chapter_1 --locked
```

The copied files under `type-exercise-starter/src/tests/` are supplied checks, not learner-owned
implementation files.

## Repository Validation

Maintainers can validate the current reference solution, minimal starter, support tool, historical
reference workspace, and book with:

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo fmt --manifest-path archived/type-exercise-ref/Cargo.toml --all --check
cargo clippy --manifest-path archived/type-exercise-ref/Cargo.toml --workspace --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path archived/type-exercise-ref/Cargo.toml --workspace --all-targets --all-features --locked
cargo check -p type-exercise-starter --lib --locked
mdbook test course
```

To validate a completed learner checkpoint, copy and run its test:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter chapter_1 --locked
cargo x copy-test --chapter 2
cargo test -p type-exercise-starter chapter_2 --locked
cargo x copy-test --chapter 3
cargo test -p type-exercise-starter chapter_3 --locked
cargo x copy-test --chapter 4
cargo test -p type-exercise-starter chapter_4 --locked
cargo x copy-test --chapter 5
cargo test -p type-exercise-starter chapter_5 --locked
cargo x copy-test --chapter 6
cargo test -p type-exercise-starter chapter_6 --locked
```

## Community

Join [skyzh's Discord server](https://skyzh.dev/join/discord) to discuss the course.

## License

The source code is licensed under Apache 2.0. See [LICENSE](./LICENSE). The mdBook text is
© 2022-2026 Alex Chi Z and licensed under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).
