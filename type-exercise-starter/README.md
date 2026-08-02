# Course Starter

This standalone crate is the learner workspace for the expression-framework course. At the
`skyzh/course-starter` ref, its implemented model is only `ScalarImpl::{Int32, String}`. You will
write every trait and associated-type connection yourself.

The repository tracks the latest stable Rust toolchain, and this crate uses Rust Edition 2024. It
is self-contained and has no dependency on `archived/`.

Run the green baseline first:

```console
cargo test --manifest-path type-exercise-starter/Cargo.toml --locked
```

Each chapter has a supplied contract test behind a feature. Its command is expected to fail to
compile before you implement the chapter and to pass afterward:

```console
cargo test --manifest-path type-exercise-starter/Cargo.toml --features chapter-1 --test chapter_1 --locked
cargo test --manifest-path type-exercise-starter/Cargo.toml --features chapter-2 --test chapter_2 --locked
```

Modify only `type-exercise-starter/src/`. Do not change the tests or enable a chapter feature by
default.
