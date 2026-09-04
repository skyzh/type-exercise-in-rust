# Build a Typed Database Expression Engine in Rust

A hand-written loop for one integer function is easy. A database expression engine also has to
borrow strings without copying, preserve nulls, read several column representations, coerce types,
and select functions at runtime.

This course builds the type families and checked boundaries that move those decisions out of each
row loop. The result is a small auto-vectorized expression engine: authors write one scalar
operation, while generic unary, binary, and ternary adapters reuse the checked batch path.

## Read the course

The complete [published course](https://skyzh.github.io/type-exercise-in-rust/) contains ten
cumulative chapter checkpoints grouped into seven teaching days. Work through the checkpoints in
order; each extends the same starter and keeps the earlier supplied tests green. Plan about 18–24
focused hours in total: two to three hours for a single-chapter day and three to four hours for a
paired day. Newer Rust learners should expect to take longer.

## What you will build

The seven teaching days follow the engine from physical values to its outer async boundary:

1. **Physical type families** (Chapter 1) connects owned and borrowed scalars, arrays, builders,
   logical types, and checked erased values.
2. **Lazy column views** (Chapter 2) reads arrays, constants, typed nulls, and Indexed views
   without materializing another column.
3. **Shared typed evaluation** (Chapter 3) lifts unary, binary, and ternary scalar operations over
   complete batches.
4. **Variable-width publication and common shapes** (Chapters 4–5) publishes string rows
   transactionally, then specializes the common Array and Constant combinations.
5. **Exceptional semantics and batch erasure** (Chapters 6–7) isolates one raw Int32 lane,
   preserves fallible and nullable semantics, and erases only the complete batch.
6. **Logical binding and one-level Lists** (Chapters 8–9) selects physical factories once and
   extends the storage model without recursive nesting.
7. **Thread-safe async evaluation** (Chapter 10) shares factories across threads and exposes one
   future per batch without moving asynchronous work into the row loop.

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
