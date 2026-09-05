# Build a Typed Database Expression Engine in Rust

A hand-written loop for one integer function is easy. A database expression engine also has to
borrow strings without copying, preserve nulls, read several column representations, coerce types,
and select functions at runtime.

This course builds the type families and checked boundaries that move those decisions out of each
row loop. The result is a small auto-vectorized expression engine: authors write one scalar
operation, while generic unary, binary, and ternary adapters reuse the checked batch path.

## Read the course

The complete [published course](https://skyzh.github.io/type-exercise-in-rust/) contains five
modules and ten cumulative, independently testable checkpoints. Work through them in order; each
extends the same starter and keeps the earlier supplied tests green. Plan roughly half a day for
each checkpoint. An experienced Rust learner can finish in about five working days, while newer
learners should expect to take longer.

## What you will build

The five modules follow the engine from physical values to its outer async boundary:

- **Type families and nullable views** (Checkpoints 1–2): connect owned and borrowed scalars,
  arrays, builders, logical types, and checked Array, Constant, Indexed, and typed-null views.
- **Shared evaluation and transactional strings** (Checkpoints 3–4): lift scalar operations over
  batches and publish variable-width rows without exposing partial writes.
- **Shape specialization and binary semantics** (Checkpoints 5–6): specialize common column
  shapes while preserving fallback behavior, then separate total, fallible, and nullable-aware
  binary evaluation.
- **Runtime erasure and the physical catalog** (Checkpoints 7–8): erase whole typed expressions
  behind a checked batch boundary and build a discoverable catalog of concrete operations.
- **Logical binding, one-level Lists, and batch async** (Checkpoints 9–10): bind logical calls to
  physical expressions, add checked nullable List storage, and defer one complete batch in a
  borrowing future.

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

Checkpoint 10 is the terminal unit. After completing it, `cargo x copy-test --chapter 10` copies
the complete cumulative supplied contract.

## Questions and feedback

Join [skyzh's Discord server](https://skyzh.dev/join/discord) for discussion. For a concrete bug or
improvement, open an [issue or pull request](https://github.com/skyzh/type-exercise-in-rust).

## License

The source code is licensed under Apache 2.0. See [LICENSE](./LICENSE). The mdBook text is
© 2022-2026 Alex Chi Z and licensed under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).
