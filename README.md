# Build a Typed Database Expression Engine in Rust

A hand-written loop for one integer function is easy. It stops scaling when the engine must also
borrow strings without copying, preserve nulls, read several column encodings, coerce types, and
select functions at runtime.

This course builds the type families and checked boundaries that move those decisions out of each
row loop. The result is a small vectorized expression engine whose generic code has a visible
payoff: new types, operators, and arities reuse the same execution path.

## Read the course

Read the [published course](https://skyzh.github.io/type-exercise-in-rust/). Work through the
chapters in order; each chapter extends the same starter and keeps earlier supplied tests green.
The book's `SUMMARY.md` is the sole ordered chapter list.

## What you will build

The currently published Chapters 1–12 build these outcomes:

- Connect logical types, owned values, borrowed values, physical arrays, and checked erased enums.
- Read nullable arrays, constants, and Indexed views without materializing a new column.
- Apply unary, binary, and ternary scalar functions through generic, erased, and bound interfaces.
- Promote supported numeric pairs for `+`, `-`, `*`, `/`, comparisons, and string `contains`.
- Represent one-level Lists with checked offsets and independent outer and child nullability.
- Preserve the same results and errors through one representative fast path.

A batch-level asynchronous adapter is reserved for future, non-required Day 13 work.
It is not part of the currently published course.

The course deliberately stops short of Decimal arithmetic, casts, and rounding, implicit
narrowing or lossy casts, nested or list-producing functions, concrete four- and five-input
builtins, exhaustive fast paths, an aggregate engine, and per-row futures.

## Start the exercises

You should know ordinary Cargo use, enums, traits, references, and `Option`. Follow the
[environment setup](https://skyzh.github.io/type-exercise-in-rust/setup.html), then work only in
`type-exercise-starter`:

```console
git fetch origin
git switch --create course-work --track origin/main
cargo check -p type-exercise-starter --lib --locked
```

Choose another branch name if `course-work` already exists. The starter baseline should compile.
Do not inspect `type-exercise/` or `archived/` while solving an exercise.

## How chapter tests work

Copy the cumulative supplied contract when you are ready to start a chapter:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter chapter_1 --locked
```

The first focused run should fail because the new behavior is missing. Read the copied destination
under `type-exercise-starter/src/tests/`, implement the named API in other starter files, and rerun
the same command until it passes. Never edit copied tests or `src/tests.rs`; keep all earlier copied
chapters green.

## Questions and feedback

Join [skyzh's Discord server](https://skyzh.dev/join/discord) for discussion. For a concrete bug or
improvement, open an [issue or pull request](https://github.com/skyzh/type-exercise-in-rust).

## License

The source code is licensed under Apache 2.0. See [LICENSE](./LICENSE). The mdBook text is
© 2022-2026 Alex Chi Z and licensed under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).
