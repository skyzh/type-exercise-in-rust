# Build a Typed Database Expression Engine in Rust

A hand-written loop for one integer function is easy. A database expression engine also has to
borrow strings without copying, preserve nulls, read several column representations, coerce types,
and select functions at runtime.

This course builds the type families and checked boundaries that move those decisions out of each
row loop. The result is a small auto-vectorized expression engine: authors write one scalar
operation, while generic unary, binary, and ternary adapters reuse the checked batch path.

## Read the course

The complete [published course](https://skyzh.github.io/type-exercise-in-rust/) contains seven
modules and fourteen independently testable labs. Work through the labs in order; each extends the
same starter and keeps the earlier supplied tests green. Plan roughly half a day for each lab. An
experienced Rust learner can finish in about seven working days, while newer learners should
expect to take longer.

## What you will build

The seven modules follow the engine from physical values to its outer async boundary:

- **Type families** (Chapters 1–2): connect owned and borrowed scalars, arrays, builders,
  logical types, and checked erased values.
- **Borrowed columns and first batch evaluation** (Chapters 3–4): read arrays, constants,
  typed nulls, and Indexed views without materializing another column, then lift one scalar
  operation over a batch.
- **Generic numeric evaluation** (Chapters 5–6): separate lossless promotion from typed kernel
  selection and share unary, binary, and ternary vectorization.
- **Specialized execution and Boolean nulls** (Chapters 7–8): select a dense fixed-width path
  once per batch while preserving the general fallback, then implement SQL three-valued Boolean
  logic.
- **Runtime expressions and variable-width output** (Chapters 9–10): erase whole typed
  expressions and build string results transactionally.
- **Logical binding and nested storage** (Chapters 11–12): bind runtime names to physical
  expressions and extend the storage model to one-level Lists.
- **Thread-safe and async boundaries** (Chapters 13–14): share logical factories across threads
  and expose one future per batch without moving asynchronous work into the row loop.

The course deliberately stops short of Decimal arithmetic, casts and rounding, lossy coercions,
nested or list-producing functions, exhaustive fast paths, an aggregate engine, and per-row
futures.

## Start the exercises

You should know ordinary Cargo use, enums, traits, references, and `Option`. Follow the
[environment setup](https://skyzh.github.io/type-exercise-in-rust/setup.html), then work only in
`type-exercise-starter`:

```console
git fetch origin
git switch --create course-work --track origin/main
cargo check -p type-exercise-starter-expr --lib --locked
```

Choose another branch name if `course-work` already exists. The starter baseline should compile.
Do not inspect `type-exercise/` or `archived/` while solving an exercise.

## How chapter tests work

Copy the cumulative supplied contract when you are ready to start a chapter:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter-supplied-tests chapter_1 --locked
```

The first focused run should fail because the new behavior is missing. Read the copied destination
under `type-exercise-starter/supplied-tests/src/`, implement the named API in other starter files, and rerun
the same command until it passes. Never edit copied tests or `supplied-tests/src/lib.rs`; keep all earlier copied
chapters green.

## Questions and feedback

Join [skyzh's Discord server](https://skyzh.dev/join/discord) for discussion. For a concrete bug or
improvement, open an [issue or pull request](https://github.com/skyzh/type-exercise-in-rust).

## License

The source code is licensed under Apache 2.0. See [LICENSE](./LICENSE). The mdBook text is
© 2022-2026 Alex Chi Z and licensed under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).
