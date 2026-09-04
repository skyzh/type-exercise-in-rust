# Course checkpoints

Each `chapter-NN` directory is an authoritative, cumulative, runnable end-of-chapter snapshot.
Its `*-expr` facade depends in one direction on its sibling `*-core` package. Compare adjacent
chapter directories to see exactly what the chapter adds.

All ten chapter snapshots use the same layout. Each snapshot includes every completed earlier
chapter and omits declarations owned by a later chapter.

Run a snapshot's complete contract from the repository root:

```console
cargo test -p type-exercise-checkpoint-NN-supplied-tests --locked
cargo test -p type-exercise-checkpoint-NN-expr --lib --locked
cargo check -p type-exercise-checkpoint-NN-core --locked
```

Each core package owns storage, views, and evaluator templates under `core/src/`; the facade owns
concrete operation instantiations under `expr/src/`. The supplied-test package links the cumulative
canonical chapter prefix through that snapshot.

An earlier chapter intentionally lacks future APIs, but it is not allowed to be stale for the
boundary it teaches. Use its tests and its adjacent diff; do not substitute the maintained
reference when reviewing a checkpoint.
