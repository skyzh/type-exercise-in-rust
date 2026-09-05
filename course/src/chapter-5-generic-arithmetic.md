# Checkpoint 5: Specialize Common Column Shapes

The Checkpoint 3 fallback calls `ColumnView::get(row)`, so Array, Constant, and Indexed inputs all
behave correctly. Its flexibility has a cost: every input selects its representation again on
every row. In this checkpoint, you will choose the common shapes once per batch and keep the typed
fallback for everything else.

Start from your completed Checkpoint 4 workspace, copy the cumulative tests, and run the focused
test before editing:

```console
cargo x copy-test --chapter 5
cargo test -p type-exercise-starter-supplied-tests chapter_5 --locked
```

The first run should fail only because the three auto-vectorization adapters do not exist yet.

## Give one loop a concrete input shape

In `core/src/column.rs`, let the core expression module inspect the private typed representation
enum. Keep that enum crate-private: callers still construct checked `ColumnViewImpl` values and
cannot bypass validation.

Add private Array and Constant accessors in `core/src/expression.rs`. Each accessor exposes the same
typed `len` and nullable `get` operations, but its concrete type is selected before the loop begins.
The loop remains generic over the accessor and no longer matches a representation at every row.

Build these public adapters around those loops:

- `auto_vectorize_unary` specializes Array and Constant; Indexed uses the existing typed fallback.
- `auto_vectorize_binary` specializes Array/Array, Array/Constant, Constant/Array, and
  Constant/Constant; any Indexed input uses the fallback.
- `auto_vectorize_ternary` specializes Array/Array/Array; every other combination uses the
  fallback.

Use the same generic scalar relationships as `evaluate_unary`, `evaluate_binary`, and
`evaluate_ternary`. The input families may differ, and the output family belongs to the generic
output scalar. Validate physical types and equal lengths before selecting a shape.

## Preserve one behavior across every path

A concrete loop must keep the fallback's semantics:

1. read borrowed typed values from each input;
2. call the scalar function only when every required input is non-null;
3. append null otherwise; and
4. return a new owned output array.

Keep the fallback because Indexed inputs still need indirect lookup. Specialize Array and Constant
inputs for unary and binary expressions, along with the common ternary case where all three inputs
are Arrays. Covering all 27 Array/Constant/Indexed ternary combinations would add a lot of code
without introducing new behavior.

Run the focused and cumulative contracts:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_5 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
```

The three Checkpoint 5 tests compare nullable owned results across Array and Constant combinations,
mixed scalar families, Indexed and non-dense ternary fallback, and invalid types and lengths.
Together with Checkpoints 1–4, the cumulative suite has 17 tests. Because callers see results and
errors rather than an internal route, both specialized and fallback paths must keep the same
contract.

The next checkpoint keeps that contract while separating operations that are total, fallible, or
nullable-aware.

{{#include copyright.md}}
