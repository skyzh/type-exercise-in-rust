# Course Starter

This is the learner workspace for the expression-framework course. Its only implemented data
model is `ScalarImpl::{Int32, String}`. You will write every trait, associated-type connection,
array, and column view yourself.

Run the compiling baseline from the repository root:

```console
cargo check -p type-exercise-starter --lib --locked
```

Each chapter provides tests through the repository's `xtask`, following the Mini-LSM course
workflow. Copy a test only when you are ready to validate that chapter:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter chapter_1 --locked
```

The command copies the selected test from the reference crate into `src/tests/` and regenerates
`src/tests.rs`. Do not edit those copied files. Implement the required public API in the remaining
files under `src/`, without reading `../type-exercise/` or `../archived/`.
