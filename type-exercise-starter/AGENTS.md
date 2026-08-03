# Type Exercise Starter Instructions

These instructions apply to work inside `type-exercise-starter/`.

## Solution boundary

- This directory is the learner workspace. Implement course tasks here without reading, searching, diffing, or copying the reference solution in `../type-exercise/` or the historical implementations in `../archived/`.
- The only permitted reference-to-starter operation is the repository-root command `cargo x copy-test --chapter <N>`. Run it without opening its source test first. After it copies the test into `src/tests/`, you may read the copied destination.
- Do not inspect Git history or an online copy to reconstruct a solution.

## Starter and tests

- The untouched starter must remain a compiling crate whose only implemented data model is `ScalarImpl::{Int32, String}`.
- Implement chapter work under `src/`, but do not modify `src/tests.rs` or files under `src/tests/`; those are supplied checks managed by `cargo x copy-test`.
- Preserve the public names and behavior required by the copied tests. Do not weaken, skip, delete, or rewrite tests to make an implementation pass.
- Keep the implementation in safe Rust and do not add dependencies, macros, generated code, expression APIs, or scalar variants before the chapter that introduces them.

## Working protocol

- Consult the current course chapter, starter interfaces, copied tests, compiler diagnostics, and official Rust documentation.
- Make the smallest change that satisfies the current chapter boundary and keep earlier copied chapter tests green.
- Treat test failures as evidence about the public contract, not permission to inspect the reference implementation.
