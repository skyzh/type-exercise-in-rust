# Course Starter

This is the only learner implementation workspace. It compiles before any exercise work. The
published course requires Checkpoints 1–10 at their implementation locations under
`expr/src/` and `core/src/`: existing declarations have concise checkpoint documentation, while future declarations
and signatures stay commented until their chapter tells you to uncomment and implement them.
The sibling facade and core packages exist from Checkpoint 1; Checkpoint 10 async scaffolds stay commented until
the final chapter tells you to implement them.

From the repository root, verify the untouched baseline:

```console
cargo check -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
cargo test -p type-exercise-starter-expr --lib --locked
```

When a chapter tells you to copy its contract, run the exact command without opening the source
test first:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter-supplied-tests chapter_1 --locked
```

The copy is cumulative and changes supplied tests only; it never parses or rewrites learner-owned
source. Follow the chapter to the local checkpoint comment, uncomment the named skeleton when
needed, and implement it. You may read the copied file under `supplied-tests/src/`, but do not edit it or
`supplied-tests/src/lib.rs`. Work in the other files under `expr/src/` and `core/src/`, keep earlier tests green, and do not inspect
`../type-exercise/` or `../archived/` for solutions.
