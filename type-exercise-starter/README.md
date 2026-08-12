# Course Starter

This is the only learner implementation workspace. It compiles before any exercise work. The
published course currently requires Day 1–13 checkpoints at their implementation locations under
`src/`: existing declarations have concise Day/checkpoint documentation, while future declarations
and signatures stay commented until their chapter tells you to uncomment and implement them.
Day 13 async scaffolds stay commented until the final chapter tells you to implement them.

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

The copy is cumulative and changes supplied tests only; it never parses or rewrites learner-owned
source. Follow the chapter to the local Day/checkpoint comment, uncomment the named skeleton when
needed, and implement it. You may read the copied file under `src/tests/`, but do not edit it or
`src/tests.rs`. Work in the other files under `src/`, keep earlier tests green, and do not inspect
`../type-exercise/` or `../archived/` for solutions.
