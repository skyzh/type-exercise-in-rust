# Course checkpoints

Each `chapter-NN` directory is the authoritative, cumulative, runnable snapshot for Checkpoint
`NN`. Its `*-expr` facade under `expr/` depends in one direction on its sibling `*-core` package
under `core/`. Compare adjacent directories to see exactly which declarations, bounds, and
implementations the checkpoint adds. `chapter-10` is the terminal snapshot.

Run a snapshot's own facade contract from the repository root:

```console
cargo test -p type-exercise-checkpoint-NN-supplied-tests --locked
cargo test -p type-exercise-checkpoint-NN-expr --lib --locked
cargo check -p type-exercise-checkpoint-NN-core --locked
```

Each core package owns its sources under `core/src/`; the facade owns concrete operations under
`expr/src/`. The supplied-test package links the cumulative canonical chapter prefix for that
checkpoint.

An earlier checkpoint intentionally lacks future APIs, but it is not allowed to be stale for the
boundary that checkpoint teaches. Use its tests and its adjacent diff; do not substitute the
maintained reference when reviewing a checkpoint.
