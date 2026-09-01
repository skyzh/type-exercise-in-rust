{{#include wip-banner.md}}

# Chapter 8: Implement Three-Valued Boolean Logic

Strict arithmetic can skip its scalar function whenever an input is null. SQL Boolean logic is
different: `FALSE AND NULL` is false, and `TRUE OR NULL` is true. Null therefore participates in
the scalar semantics of `AND` and `OR`, while `NOT` remains strict.

The reusable core already supports both contracts. This chapter keeps the choice in the facade:
three small scalar functions define SQL truth, and the expression builder selects the matching
core evaluator once per batch.

## Checkpoint 1: own the truth table as scalar code

Begin from completed Chapter 7:

```console
cargo x copy-test --chapter 8 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_8 --locked
```

Enable only the private `expr/src/boolean.rs` module in `expr/src/lib.rs`, then implement these
crate-visible scalar functions:

```rust,ignore
pub(crate) fn not(value: bool) -> bool;
pub(crate) fn and(left: Option<bool>, right: Option<bool>) -> Option<bool>;
pub(crate) fn or(left: Option<bool>, right: Option<bool>) -> Option<bool>;
```

Keep the functions concise. `NOT` flips a present Boolean. `AND` returns false as soon as either
side is false, true only for two true inputs, and null otherwise. `OR` returns true as soon as
either side is true, false only for two false inputs, and null otherwise.

The supplied test owns the exhaustive 21 rows. Do not export a production truth-table constant or
`BooleanTruthRow` merely to make the test convenient; production code owns behavior, and tests own
enumerated examples.

## Checkpoint 2: select operation and null semantics once

Copy the completed stage:

```console
cargo x copy-test --chapter 8 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_8 --locked
cargo test -p type-exercise-starter-expr --lib --locked
```

Add `BooleanOperator::{And, Or, Not}` and `build_boolean_expression`. The builder selects one
function before row evaluation:

- `Not` delegates to the strict unary evaluator;
- `And` delegates to the nullable-aware binary evaluator with the `and` scalar function; and
- `Or` delegates to the same evaluator with `or`.

Now uncomment the `pub use boolean::*` line in `expr/src/lib.rs` so the completed expression surface is
available to later chapters. The scalar helpers remain crate-visible implementation details.

Keep operator selection outside the shared loops. A null-policy enum tested per row would make
the core depend on Boolean semantics and would put dispatch back into the hot path. The selected
function itself may branch because those branches *are* SQL Boolean semantics, not operation
dispatch.

The eight focused tests cover all truth rows, array evaluation, absorption rules, strict `NOT`,
arity and metadata, structural operation selection, and unchanged type/length validation.

## Inspect the boundary

There are now two kinds of control flow and they should not be confused:

1. batch-level dispatch chooses `AND`, `OR`, or `NOT` once;
2. scalar-level matching implements the chosen SQL truth table for one row.

The core crate knows only whether a callback accepts values or `Option` values. The facade owns the
meaning of `FALSE AND NULL`. This one-way ownership is why adding a different nullable-aware
operation does not require another core loop.

Next: [Chapter 9 erases complete typed expressions at runtime](./chapter-9-binding-coercion.md).

{{#include copyright.md}}
