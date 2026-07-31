# Day 3: Specialize and Measure Primitive Loops

Take two all-valid `i32` arrays with 65,536 rows. The general evaluator still asks whether both
inputs are valid on every row and pushes one validity bit for every output. Those checks are
correct for nullable data, but they can prevent a simple addition or comparison loop from
vectorizing.

Today you will select an all-valid primitive loop once per batch, preserve the general path for
every other case, and compare both paths with hand-written loops over the same storage.

## Starting Point and Result

Day 2 binds a logical signature to one typed kernel. The selected generated evaluator accepts every
Day 1 representation and propagates nulls correctly.

After this day:

- `PrimitiveArray<T>` caches its null count;
- `as_non_null` returns a checked borrowed proof before exposing a dense value slice;
- `PrimitiveBinaryExpression` specializes all-valid primitive arrays and non-null constants;
- nullable arrays, null constants, and dictionaries use the existing general evaluator; and
- Criterion compares generated and hand-written loops with identical inputs and outputs.

This day does not add a new `ColumnView` variant or relax null semantics. It also does not claim
that every expression should have a specialized loop.

## Observe the Branch Before Naming the Optimization

The general loop is conceptually:

```rust,ignore
for row in 0..len {
    match (left.get(row), right.get(row)) {
        (Some(left), Some(right)) => output.push(Some(func(left, right))),
        _ => output.push(None),
    }
}
```

For an all-valid batch, both `get` calls always take the same branch. A hand-written dense loop can
instead zip two `&[i32]` slices, call the function, collect values, and initialize an all-valid
bitmap once.

The optimization is therefore batch selection, not a different expression:

```text
                    primitive inputs
                           |
                 both representations eligible?
                    /                  \
                  no                    yes
                  |                      |
          general nullable loop   both values proven valid?
                                         /        \
                                       no          yes
                                       |            |
                              general nullable   dense loop
```

Dictionary views remain on the general path because a dictionary key may be null and its lookup is
not a contiguous scan of the logical rows.

## The Fast-Path Contract

Implement the contract in `expr-common/src/array/primitive_array.rs` and
`expr-template/src/primitive.rs`.

1. `null_count` equals the number of false bits in the primitive array's validity bitmap.
2. Every constructor and builder operation maintains that count.
3. `as_non_null` exposes the value slice only when `null_count == 0`.
4. `PrimitiveArray::from_values` creates an array of the same length whose rows are all valid.
5. The specialized and general paths produce identical values and nulls for every supported input.
6. All-valid array/array, array/constant, constant/array, and constant/constant inputs may use the
   dense loop.
7. A nullable input, null constant, or dictionary input must fall back to the general evaluator.
8. The planner uses `PrimitiveBinaryExpression` only for the numeric addition and primitive
   comparison families. String, decimal-specific, list, and other custom kernels keep the general
   template unless measured evidence justifies another specialization.

The proof wrapper and fallback behavior are course rules. Iterator combinators versus an indexed
loop inside the dense path are implementation choices that benchmark evidence may guide.

## Why the Proof Is a Wrapper

Returning `&[T]` directly from a method named `values` would let callers forget why validity checks
are safe to skip. `NonNullPrimitiveArray<'a, T>` records that the check occurred:

```rust,ignore
pub fn as_non_null(&self) -> Option<NonNullPrimitiveArray<'_, T>> {
    (self.null_count == 0).then_some(NonNullPrimitiveArray(self))
}
```

The wrapper borrows the original array, so its dense slice cannot outlive the bitmap that justified
it. It is not a logical type and does not cross the expression boundary.

Work through `[Some(4), None, Some(9)]`: the builder stores three values, three validity bits, and
`null_count == 1`. `as_non_null` returns `None`, so the general loop produces the null result. For
`[Some(4), Some(0), Some(9)]`, the count is zero and the dense loop sees `[4, 0, 9]`.

## Preserve One Framework

`PrimitiveBinaryExpression` first converts the erased views to typed `ColumnView`s and checks their
lengths. It then tries the eligible dense combinations. If a proof or representation check fails,
it delegates to:

```rust,ignore
BinaryExpression::<I1, I2, O, _>::new(&self.func).eval_views(left, right)
```

Delegation matters. Copying the nullable loop into the specialization would create two
implementations of null propagation, dictionary reads, and error behavior.

## Benchmark Like-for-Like Work

The benchmark in `expr-impl/benches/expression.rs` fixes the row count at 65,536 and compares:

- generated array/array, array/constant, and dictionary/array evaluation with hand-written loops
  over the same representations;
- the general nullable template, primitive fast path, and hand-written dense loop for all-valid
  comparison; and
- the same three paths for addition.

Each case materializes the same output array. Criterion uses `black_box`, reports element
throughput, and runs release-mode code. The benchmark README treats a persistent gap as a reason to
inspect generated code, not as a correctness result.

A quick run on one development machine observed the primitive comparison path within about 1% and
the addition path within about 7% of their hand-written dense baselines. Those numbers are
observations, not portable thresholds: CPU, compiler, power state, and background load can change
absolute timings.

## Implementation Checkpoints

Work in this order:

1. Add and test `null_count`, `has_nulls`, `as_non_null`, and `from_values`.
2. Implement the primitive expression with the existing general evaluator as fallback.
3. Use the primitive expression for generated numeric addition and primitive comparisons.
4. Add like-for-like hand-written baselines for all Day 1 representations.
5. Add the general-template and dense-loop comparisons for all-valid arrays.

Keep changes inside primitive-array storage, the primitive expression adapter, the binder's kernel
selection, and benchmark files. Do not specialize dictionary access or add unsafe unchecked reads.

## Verify the Day

Run correctness checks first:

```console
cargo test -p expr-common primitive_array --locked
cargo test -p expr-template primitive --locked
cargo test -p expr-impl --locked
```

Then collect informational performance evidence:

```console
cargo bench -p expr-impl --bench expression
```

The correctness tests should prove both the all-valid case and nullable fallback. The benchmark
should report all generated, general-template, and hand-written cases; timing differences do not
change the correctness verdict.

Before moving on, explain:

- which invariant makes `values()` safe to use without per-row validity checks;
- why a dictionary stays on the general path;
- how the fast path preserves Day 1 and Day 2 errors and null behavior; and
- why the hand-written baseline uses the same storage and output builder.

Next, [strengthen the Rust boundaries around the same framework](./day-4-rust-boundaries.md).

{{#include copyright.md}}
