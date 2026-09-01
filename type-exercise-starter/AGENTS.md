# Type Exercise Starter Instructions

These instructions apply to work inside `type-exercise-starter/`.

## Solution boundary

- This directory is the learner workspace. Implement course tasks here without reading, searching, diffing, or copying the reference solution in `../type-exercise/` or the historical implementations in `../archived/`.
- The only permitted reference-to-starter operation is the repository-root command `cargo x copy-test --chapter <N>`. Run it without opening its source test first. After it copies the test into `supplied-tests/src/`, you may read the copied destination.
- Do not inspect Git history or an online copy to reconstruct a solution.

## Starter and tests

- The untouched starter must remain a compiling crate. Published and required Day 1–14 ownership
  lives beside the actual implementation location: existing declarations carry Day/checkpoint
  documentation, and future declarations/signatures remain commented until their chapter asks you
  to uncomment them. Day 14 async scaffolds remain commented until the final chapter asks you to
  implement them.
- Implement chapter work under `expr/src/` and `core/src/`, but do not modify `supplied-tests/src/lib.rs` or files under `supplied-tests/src/`; those are supplied checks managed by `cargo x copy-test`.
- Preserve the public names and behavior required by the copied tests. Do not weaken, skip, delete, or rewrite tests to make an implementation pass.
- Keep the implementation in safe Rust. Do not uncomment or materialize expression, binder, List,
  iterator, or async skeletons before their chapter.

## Working protocol

- Consult the current course chapter, starter interfaces, copied tests, compiler diagnostics, and official Rust documentation.
- Make the smallest change that satisfies the current chapter boundary and keep earlier copied chapter tests green.
- Treat test failures as evidence about the public contract, not permission to inspect the reference implementation.
