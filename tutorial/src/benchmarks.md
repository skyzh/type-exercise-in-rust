# Benchmarks and Next Steps

Type-level elegance is not evidence of runtime performance. The repository includes Criterion
benchmarks that compare generated typed expressions with hand-written monomorphic loops over the
same arrays and builders.

Run them in release mode:

```console
cargo bench -p expr-impl --bench expression
```

{{#include copyright.md}}

## Baselines

The benchmark evaluates nullable `i32 <= i32` over 65,536 rows in three layouts:

| Input layout | Generated path | Hand-written baseline |
| --- | --- | --- |
| array / array | `BinaryExpression` over two array views | direct zip of two `I32Array` iterators |
| array / constant | array and constant accessors | direct array loop closing over one `i32` |
| dictionary / array | dictionary and array accessors | direct index lookup plus array access |

Every case materializes the same `BoolArray`. The comparison therefore isolates framework and view
dispatch overhead instead of comparing unrelated memory layouts.

An Apache Arrow kernel would be another useful system-level baseline, but it would also compare
bitmap formats, buffer ownership, kernel algorithms, and dependency configuration. Start with the
hand-written same-representation baseline. Add Arrow when you want to evaluate interoperability or
the complete physical implementation.

## Current Result

On the development machine used for this revision (Apple Silicon, Rust 1.94), a short Criterion run
measured approximately:

| Case | Generated | Hand-written | Difference |
| --- | ---: | ---: | ---: |
| array / array | 190 µs | 185 µs | about 3% slower |
| array / constant | 159 µs | 143 µs | about 11% slower |
| dictionary / array | 238 µs | 193 µs | about 23% slower |

Treat these numbers as a local observation, not a portable guarantee. CPU, compiler, sample size,
and background load change microbenchmarks. The important result is diagnostic: matching the view
enum inside every row originally made the array/array path roughly 45% slower. Moving representation
dispatch outside the loop reduced that gap to a few percent.

The benchmark README proposes investigating persistent array/array regressions above 15%.
Dictionary access naturally performs an extra nullable index lookup; future work can inspect bounds
check elimination, selection-vector composition, and dictionary-specialized kernels.

## Read the Results Carefully

When a benchmark changes:

1. compare the generated and hand-written algorithms for semantic equality;
2. verify that `black_box` prevents constant folding without hiding allocation;
3. inspect whether a runtime match moved inside the loop;
4. use a profiler or generated assembly before guessing; and
5. measure end-to-end query effects before accepting a more complex design.

Do not add a flaky wall-clock assertion to the unit tests. Criterion reports distributions; CI
performance testing requires controlled runners and a separate regression policy.

## Next Steps

The framework is intentionally small. Productive extensions include:

- a selection or sparse column view;
- Arrow adapters implementing the typed accessor interface;
- cast expressions inserted by a richer binder;
- function properties for null behavior, commutativity, and monotonicity;
- unary, variadic, and writer-style registration helpers;
- decimal promotion that accounts for precision and scale;
- stateful kernels for regular expressions and JSON paths;
- fallible scalar functions with row-level error reporting; and
- fused expression evaluation that avoids intermediate output arrays.

Keep the organizing principle: use generic generation where a genuine family shares one algorithm,
and keep data-type-specific behavior explicit everywhere else.

## Final Validation

Before publishing changes to the framework, run:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
mdbook build tutorial
cargo bench -p expr-impl --bench expression
```
