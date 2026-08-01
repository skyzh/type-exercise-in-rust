# Build a Database Expression Framework in Rust

This course builds a small vectorized database expression framework from an explicit Rust starter.
The starter contains one owned scalar enum and no trait relationships. You will connect owned
values, borrowed values, and arrays yourself before adding runtime-erased column views.

The current review boundary contains two chapters:

1. connect `i32`, `String`, their borrowed forms, and their physical arrays with traits and generic
   associated types;
2. read arrays, repeated constants, and dictionary-encoded values through one borrowed interface.

Later chapters will vectorize scalar functions, erase and generate expressions, bind logical
signatures, specialize primitive loops, strengthen Rust type boundaries, and add a batch-level
async adapter. These topics are not forced into a fixed calendar.

## Start Here

The starter is a real standalone crate at
[`type-exercise-starter`](./type-exercise-starter). Begin from the
`skyzh/course-starter` branch so the chapter solutions are not present:

```console
git fetch origin
git switch --create course-work --track origin/skyzh/course-starter
cargo test --manifest-path type-exercise-starter/Cargo.toml --locked
mdbook serve course --open
```

The repository root is a course container, not a Cargo workspace. `type-exercise-starter/` is the
only learner workspace. Everything under [`archived/`](./archived) predates this course structure
and is neither a starter dependency nor part of the chapter work. Keep it out of the learner path
until you finish the chapter reflection.

Read the mdBook source in [`course`](./course), beginning with the
[preface](./course/src/preface.md), [setup](./course/src/setup.md), and
[Chapter 1](./course/src/chapter-1-type-connections.md). Continue with
[Chapter 2](./course/src/chapter-2-column-views.md).

## Repository Validation

Maintainers can validate the maintained archived reference workspace, the starter baseline, and
the book with:

```console
cargo fmt --manifest-path archived/type-exercise-ref/Cargo.toml --all --check
cargo clippy --manifest-path archived/type-exercise-ref/Cargo.toml --workspace --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path archived/type-exercise-ref/Cargo.toml --workspace --all-targets --all-features --locked
cargo test --manifest-path type-exercise-starter/Cargo.toml --locked
mdbook test course
```

On a completed solution checkpoint, also run its gated contract test:

```console
cargo test --manifest-path type-exercise-starter/Cargo.toml --features chapter-1 --test chapter_1 --locked
cargo test --manifest-path type-exercise-starter/Cargo.toml --features chapter-2 --test chapter_2 --locked
```

The second command implies the Chapter 1 feature. On the scalar-only starter checkpoint, these
featured contracts are expected to fail to compile until the matching implementation exists.

## Community

Join [skyzh's Discord server](https://skyzh.dev/join/discord) to discuss the course.

## License

The source code is licensed under Apache 2.0. See [LICENSE](./LICENSE). The mdBook text is
© 2022-2026 Alex Chi Z and licensed under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/).
