# Reference Solution

This crate contains the maintained reference implementation for the current course checkpoints.
Learners work only in `type-exercise-starter/`; they should not inspect this directory while
solving a chapter.

The canonical chapter tests live under `src/tests/`. The repository-root command
`cargo x copy-test --chapter <N>` copies the cumulative supplied tests through chapter `N` into
the starter and regenerates its test module list. It does not parse or rewrite learner source.
