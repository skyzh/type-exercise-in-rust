# Reference Solution

This crate contains the maintained reference implementation for the current course checkpoints.
Learners work only in `type-exercise-starter/`; they should not inspect this directory while
solving a chapter.

The canonical twelve-chapter tests live under `src/tests/`. The repository-root command
`cargo x copy-test --chapter <N>` copies the cumulative prefix into the starter, removes later
managed tests, and regenerates the starter's test module list.
