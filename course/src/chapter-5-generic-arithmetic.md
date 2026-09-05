# Checkpoint 5: Specialize Common Column Shapes

The Checkpoint 3 fallback calls `ColumnView::get(row)` so Array, Constant, and Indexed inputs all
behave correctly. That generality also asks the view to select its representation for every input
on every row. This checkpoint moves only common shapes into concrete loops while keeping the typed
fallback as the behavior to preserve.

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
The loop can then remain generic over the accessor without matching a representation at every row.

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

Do not remove or duplicate the fallback. Indexed inputs still need their indirect lookup, and
specializing all 27 ternary Array/Constant/Indexed combinations would add code without establishing
a new learner-visible behavior. The selective boundary here is Array and Constant for unary and
binary work, plus the common all-Array ternary case.

Run the focused and cumulative contracts:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_5 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
```

The three Checkpoint 5 tests compare nullable owned results across Array and Constant combinations,
mixed scalar families, Indexed and non-dense ternary fallback, and invalid types and lengths.
Together with Checkpoints 1–4, the cumulative suite has 17 tests. The tests observe results and
errors, not which internal route produced them.

Raw-buffer specialization, exceptional scalar semantics and concrete facades, runtime expression
erasure, the binder and registry, List evaluation, and asynchronous evaluation remain future work.

{{#include copyright.md}}
