{{#include wip-banner.md}}

# Chapter 7: Implement Three-Valued Boolean Logic

SQL engines do not stop at two truth values. A missing value makes `NULL AND FALSE` false and
`NULL OR TRUE` true, so nulls must flow into the Boolean scalar function instead of always
short-circuiting.

**Prerequisites:** Chapter 6, the checked-expression boundary, and nullable `Boolean` columns.

**By the end of this chapter, you will:**

- distinguish strict null short-circuiting from SQL's non-strict null semantics;
- implement `AND`, `OR`, and `NOT` over `TRUE`/`FALSE`/`NULL`; and
- publish one checked expression that reuses Day 6's nullable unary/binary auto-vectorizers.

```console
cargo x copy-test --chapter 7
cargo test -p type-exercise-starter chapter_7 --locked
```

The first run should fail on the missing Boolean scalar operation or expression builder.

## Two null policies

The Day 4–6 shells skip the scalar function for any strict null input: the row is null, and the
function never sees it. That is the `Strict` policy. SQL's three-valued logic needs more: a null
operand can still decide the result when the other operand is absorbing (`FALSE AND ...`, `TRUE OR
...`). That is the `NonStrict` policy, where nulls are passed to the scalar function and the truth
table decides.

## Checkpoint 1: implement the scalar truth operation

- **Target:** `type-exercise-starter/src/boolean_logic.rs::{NullEvaluationPolicy, BooleanOperator, apply_boolean}`.
- **Change:** declare both policies, the three operators, and one scalar match expression, with
  `FALSE` absorbing for `AND`, `TRUE` absorbing for `OR`, and `NOT NULL` staying null.
- **Preserve:** the supplied test—not production—owns all 21 exhaustive input/result tuples.
- **Run:** the Chapter 7 focused test.
- **Passing means:** the scalar function matches every required three-valued truth row without a
  public production table.

## Checkpoint 2: evaluate one operator

- **Target:** `type-exercise-starter/src/boolean_logic.rs::{BooleanExpression, build_boolean_expression}`.
- **Change:** validate arity (two for `AND`/`OR`, one for `NOT`), physical types, and lengths before
  any row work, then call Day 6's nullable unary or binary auto-vectorizer with `apply_boolean`.
  `build_boolean_expression` selects SQL `NonStrict`; `BooleanExpression::new` exposes `Strict`.
- **Preserve:** the row error and null behavior stay inside the checked-expression contract; the
  error representation is your readable choice.
- **Run:** the focused and cumulative tests.
- **Passing means:** evaluation reproduces the full truth table, strict short-circuits before the
  truth table, and wrong arity/type/length fail closed.

## Required and extension work

Both policies, all three operators, the scalar truth operation, and the checked builder are required.
Nested expression trees and short-circuit execution plans are extensions outside this course.

```console
cargo test -p type-exercise-starter chapter_7 --locked
cargo test -p type-exercise-starter --lib --locked
```


Next: [Chapter 8 erases typed expressions behind one object-safe boundary](./chapter-8-runtime-erasure.md).

{{#include copyright.md}}
