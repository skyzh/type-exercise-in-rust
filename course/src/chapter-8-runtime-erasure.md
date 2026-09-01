{{#include wip-banner.md}}

# Chapter 8: Implement Three-Valued Boolean Logic

Strict arithmetic can skip its scalar function whenever an input is null. SQL Boolean logic is
different: `FALSE AND NULL` is false, and `TRUE OR NULL` is true. Null therefore participates in
the scalar semantics of `AND` and `OR`, while `NOT` remains strict.

The reusable core already contains both contracts, but Chapter 7 leaves its nullable-aware helpers
private. This chapter first publishes that core boundary, then keeps the operation choice in the
facade: three small scalar functions define SQL truth, and the expression builder selects the
matching core evaluator once per batch.

## Checkpoint 1: expose the truth table through an expression

Begin from completed Chapter 7:

```console
cargo x copy-test --chapter 8 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_8 --locked
```

Open `core/src/expression.rs`. The nullable-aware unary and binary helpers already own their row
loops, but they are private in the completed Chapter 7 state. Make both functions public without
changing their bodies. The strict `evaluate_unary` helper is already public.

Enable the private `expr/src/boolean.rs` module in `expr/src/lib.rs`, then implement the three
crate-visible scalar helpers:

```rust,ignore
pub(crate) fn not(value: bool) -> bool;
pub(crate) fn and(left: Option<bool>, right: Option<bool>) -> Option<bool>;
pub(crate) fn or(left: Option<bool>, right: Option<bool>) -> Option<bool>;
```

Keep the functions concise. `NOT` flips a present Boolean. `AND` returns false as soon as either
side is false, true only for two true inputs, and null otherwise. `OR` returns true as soon as
either side is true, false only for two false inputs, and null otherwise.

Define public `BooleanOperator::{And, Or, Not}`, `BooleanExpression`, and
`build_boolean_expression`, and export that expression boundary from `expr/src/lib.rs`. Give the
expression an inherent `evaluate` method. Validate a one-element Boolean type slice for `Not` and
a two-element Boolean type slice for `And` and `Or`, then select the matching core evaluator outside
the row loop: strict unary for `Not`, nullable-aware binary for `And` and `Or`.

The supplied test sends the exhaustive 21 truth rows through the public builder,
`ColumnViewImpl`, and `evaluate`; it never imports the private helpers or depends on their names,
file, or signatures. Do not export a production truth-table constant or `BooleanTruthRow` merely
to make the test convenient: production code owns behavior, while tests own enumerated examples.

## Checkpoint 2: complete the batch contract

Copy the completed stage:

```console
cargo x copy-test --chapter 8 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_8 --locked
cargo test -p type-exercise-starter-expr --lib --locked
```

Add public `operator`, `arity`, `input_types`, and `output_type` metadata to the expression. Refactor
`evaluate` to validate with `input_types`; its evaluator selection and truth semantics remain those
from Checkpoint 1. The completed test now exercises array-backed inputs, metadata, arity/type/length
errors, and the same shared-core integration.

The scalar helpers remain crate-visible implementation details. Later chapters depend only on the
public expression surface exported at Checkpoint 1.

Keep operator selection outside the shared loops. A null-policy enum tested per row would make
the core depend on Boolean semantics and would put dispatch back into the hot path. The selected
function itself may branch because those branches *are* SQL Boolean semantics, not operation
dispatch.

The eight focused tests cover all truth rows, array evaluation, absorption rules, strict `NOT`,
arity and metadata, operation selection through public results, and unchanged type/length
validation.

## Inspect the boundary

There are now two kinds of control flow and they should not be confused:

1. batch-level dispatch chooses `AND`, `OR`, or `NOT` once;
2. scalar-level matching implements the chosen SQL truth table for one row.

The core crate knows only whether a callback accepts values or `Option` values. The facade owns the
meaning of `FALSE AND NULL`. This one-way ownership is why adding a different nullable-aware
operation does not require another core loop.

Next: [Chapter 9 erases complete typed expressions at runtime](./chapter-9-binding-coercion.md).

{{#include copyright.md}}
