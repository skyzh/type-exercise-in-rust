# Reference Solution

The `type-exercise-expr` facade under `expr/` and its sibling `type-exercise-core` dependency
under `core/` contain the maintained reference implementation for the current course checkpoints.
Learners work only in `type-exercise-starter/`; they should not inspect this directory while
solving a chapter.

The canonical chapter tests live under `supplied-tests/src/`. The repository-root command
`cargo x copy-test --chapter <N>` copies the cumulative supplied tests through chapter `N` into
`type-exercise-starter/supplied-tests/src/` and regenerates that package's module list. It does not
parse or rewrite learner source.
