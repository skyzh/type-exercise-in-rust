# Chapter 6: Specialize Primitive Loops

Chapter 5 selects `i32_add` once during binding, but its physical adapter still evaluates every
representation through `ColumnView::get` and builds one `Option<i32>` at a time. That general loop
is the right fallback. Dense primitive inputs can establish stronger facts before the first row:

```text
all-valid i32 array + all-valid i32 array
                    -> select one dense loop -> I32Array
```

This chapter adds four all-valid loop shapes without changing the logical registry, scalar rule,
null semantics, or checked error boundaries. It does extend the erased and bound expression APIs
so tests can observe which loop one batch selected.

## Starting Point and Result

Continue from your Chapter 5 implementation and copy the next supplied contract:

```console
cargo x copy-test --chapter 6
cargo test -p type-exercise-starter chapter_6 --locked
```

After this chapter, add these public pieces:

- dense primitive storage with a validity vector and cached null count;
- `NonNullPrimitiveArray`, a checked view that proves all values are valid;
- `PrimitiveBinaryExpression`, the specialized adapter for `i32, i32 -> i32`; and
- `PrimitiveLoop`, which names the selected loop; and
- `Expression::evaluate_with_loop` plus a `BoundExpression` forwarding method, which expose that
  observation through the erased catalog and logical binder.

Keep `BinaryExpression` as the general adapter. The builtin catalog should construct the
specialized adapter only for `i32_add`; `string_concat` stays on the generic path.

## Separate Values from Validity

The earlier `Vec<Option<i32>>` layout is convenient, but an all-valid primitive loop should not
branch on an `Option` for every row. Refactor `PrimitiveArray<T>` to hold:

```rust,ignore
values: Vec<T>
validity: Vec<bool>
null_count: usize
```

The two vectors always have equal length. A null slot still reserves one value position, using a
placeholder that is never observable through `Array::get`. Cache `null_count` when the builder is
finished rather than scanning the validity vector for every expression batch.

Provide `PrimitiveArray::from_values` for already-dense owned values. Provide
`PrimitiveArray::as_non_null` only when the cached count is zero. The returned
`NonNullPrimitiveArray` exposes the dense value slice while borrowing the checked array. It must
not have a public unchecked constructor.

The proof is deliberately narrow:

- zero nulls proves every value slot is logically present;
- one or more nulls refuses the proof and keeps the general representation; and
- an empty primitive array is valid and all-valid.

## Select a Loop Once per Batch

For two all-valid integer inputs there are four dense shapes:

| Left | Right | Selected loop |
| --- | --- | --- |
| array | array | `ArrayArray` |
| array | constant | `ArrayConstant` |
| constant | array | `ConstantArray` |
| constant | constant | `ConstantConstant` |

Choose the shape before iterating. Each dense loop reads plain `i32` values, applies the same
`I32Add::evaluate` scalar function, collects plain output values, and finishes with
`I32Array::from_values`. Addition must retain Chapter 3's wrapping-overflow rule.

Do not turn this into a per-row representation match. If the chosen loop still asks whether an
input is an array or constant for every row, it has not moved dispatch out of the hot loop.

Extend `Expression` with an object-safe `evaluate_with_loop` observer that returns the erased output
and `PrimitiveLoop`. Its default implementation calls the expression's ordinary `evaluate` method
and reports `General`. `PrimitiveBinaryExpression` overrides the observer; its ordinary `evaluate`
method calls the specialized observer and discards the loop label. Add the same forwarding method
to `BoundExpression` so supplied tests can prove that binding and catalog lookup selected the
primitive adapter without downcasting the trait object.

## Preserve the General Fallback

Only two representation families qualify for the dense proof:

- an `Int32` array whose cached null count is zero; and
- a non-null `Int32` constant.

Everything else delegates to the general Chapter 3 loop:

- nullable primitive arrays;
- typed null constants;
- dictionary views, even when their values array happens to be all-valid; and
- every non-primitive expression such as string concatenation.

A dictionary is intentionally a fallback because each row still needs key decoding and null-key
handling. Do not materialize it merely to reach the fast path. Its constructor already validates
all keys, and that validation belongs outside expression timing.

## Keep Error Precedence Stable

Specialization cannot weaken or reorder checked failures. The primitive adapter performs:

1. arity validation before indexing the input slice;
2. left then right physical-type validation;
3. length validation before producing output; and
4. dense-loop selection or general delegation.

This order preserves the behavior of the general adapter. A wrong physical type remains a
`TypeMismatch`; unequal valid integer inputs remain an `InputLengthMismatch`; nulls remain output
nulls rather than errors.

## Generate the Right Adapter

Chapter 4's catalog macro originally wrapped every scalar function in `BinaryExpression`. Change
the catalog entries so each entry supplies its constructed expression:

```rust,ignore
"i32_add" => PrimitiveBinaryExpression::new("i32_add", I32Add),
"string_concat" => BinaryExpression::new("string_concat", StringConcat),
```

The logical registry and accepted signatures do not change. Chapter 5 still binds `+` to the
physical name `i32_add`; the catalog now chooses its optimized adapter behind the same
`Box<dyn Expression>` boundary. Only the bound expression's observer forwarding method is new.

## Measure Without Timing Setup

The repository also contains a maintainer benchmark for the completed reference solution:

```console
cargo bench -p type-exercise --bench expression
```

This command targets `type-exercise`, not the learner's `type-exercise-starter` crate. It is
reference-only evidence and is not part of the learner completion contract.

The benchmark covers the four dense shapes plus nullable-array, null-constant, and dictionary
fallbacks. Every case uses deterministic inputs and compares three implementations:

- the general `BinaryExpression`;
- the specialized `PrimitiveBinaryExpression`; and
- a handwritten lower-bound kernel.

The general and specialized adapters receive the same `ColumnViewImpl` inputs. For dense cases,
the handwritten kernel receives preclassified slices or constant metadata outside timing; it is a
lower bound on loop work, not a peer adapter performing representation selection. Fallback
handwritten cases use the general typed-view loop. All three paths materialize the same `I32Array`,
and the harness checks equality before timing. Construct and validate dictionary keys outside the
timed closure. Otherwise an `O(n)` setup scan can be mistaken for expression work and distort the
comparison.

Treat Criterion output as an observation for that machine, toolchain, and load. The course does
not impose a portable percentage threshold. Dense specialization should be evaluated by semantic
equivalence, loop selection, and reproducible methodology first; performance numbers can vary.

## Implementation Checkpoints

1. Split primitive values from validity and cache the null count.
2. Add a checked borrowed proof for an all-valid primitive array.
3. Identify only all-valid integer arrays and non-null integer constants as dense inputs.
4. Implement all four preselected dense loops with the existing scalar function.
5. Delegate nullable, null-constant, and dictionary inputs to the general evaluator.
6. Keep string concatenation on `BinaryExpression` and binding unchanged.
7. Keep the maintainer benchmark reference-only and check output equality outside timing.

## Review Your Chapter Result

Run:

```console
cargo test -p type-exercise-starter chapter_6 --locked
cargo test -p type-exercise-starter --lib --locked
```

The Chapter 6 contract contains seven tests. They cover primitive null-count proofs, all four
dense loop shapes, wrapping addition, nullable/null-constant/dictionary fallbacks, preserved
arity/type/length errors, logical binding, and the unchanged string catalog path.

Before continuing, explain:

- why a cached null count is a proof input rather than only a statistic;
- why dictionary decoding remains in the general loop;
- why representation selection belongs before the row loop;
- why fast-path errors must match the general adapter's order; and
- why the reference-only benchmark treats a preclassified handwritten kernel as a lower bound
  instead of a portable speed threshold.

Chapter 7 will keep these runtime semantics and strengthen Rust's API boundaries around iteration,
variance, trait upcasting, and thread-safety.

{{#include copyright.md}}
