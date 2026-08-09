# Course Starter

This is the only learner implementation workspace. It begins with
`ScalarImpl::{Int32, String}` and no relationship among owned values, borrowed values, arrays, or
expressions.

From the repository root, verify the untouched baseline:

```console
cargo check -p type-exercise-starter --lib --locked
cargo test -p type-exercise-starter --lib --locked
```

When a chapter tells you to copy its contract, run the exact command without opening the source
test first:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter chapter_1 --locked
```

The copy is cumulative. The first run for a new chapter is expected to fail until you implement
its public contract. You may read the copied file under `src/tests/`, but do not edit it or
`src/tests.rs`. Work in the other files under `src/`, keep earlier tests green, and do not inspect
`../type-exercise/` or `../archived/` for solutions.
