# Course checkpoints

Each `day-NN` directory is an authoritative, cumulative, runnable end-of-day snapshot. Its
`*-expr` facade under `expr/` depends in one direction on its sibling `*-core` package under `core/`. Compare adjacent
directories to see exactly which declarations, bounds, and implementations the day adds.

Run a snapshot's own facade contract from the repository root:

```console
cargo test -p type-exercise-checkpoint-NN-supplied-tests --locked
cargo test -p type-exercise-checkpoint-NN-expr --lib --locked
cargo check -p type-exercise-checkpoint-NN-core --locked
```

Each core package owns its sources under `core/src/`; the facade owns concrete operations under
`expr/src/`. The supplied-test package links the cumulative canonical chapter prefix for that day.

An earlier day intentionally lacks future APIs, but it is not allowed to be stale for the boundary
that day teaches. Use its tests and its adjacent diff; do not substitute the maintained reference
when reviewing a checkpoint.
