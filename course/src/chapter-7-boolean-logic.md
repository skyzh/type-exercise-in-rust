{{#include wip-banner.md}}

# Chapter 7: Select Dense Fixed-Width Loops

Chapter 6 made operation authorship small: a scalar function is lifted by one reusable evaluator.
That general evaluator must still support nullable arrays, constants, typed nulls, and Indexed
views, so its row loop asks each view for an `Option` on every iteration. Fixed-width, all-valid
input has a simpler representation and deserves a simpler loop.

This chapter adds that path without weakening the general contract. One batch-level decision
selects either a dense loop or the existing nullable-aware fallback. The dense loop contains only
loads, the already selected scalar call, and output writes; it does not redispatch the operation or
recheck nullability inside each row.

## What is in the starter

Begin from completed Chapter 6 and copy the first cumulative checkpoint:

```console
cargo x copy-test --chapter 7 --checkpoint 1
cargo test -p type-exercise-starter-expr chapter_7 --locked
```

The shared storage and evaluator work belongs in the core package. The concrete `I32Add` scalar
operation remains in the expression facade. You will add:

- physical `Nullability` observations for erased column views;
- checked recovery of a non-null fixed-width array view;
- four dense input-shape loops for array/array, array/constant, constant/array, and
  constant/constant; and
- a `PrimitiveBinaryExpression` that chooses one loop before traversing rows.

Nullable arrays, null constants, and Indexed views must keep using the general Chapter 6 path.

## Checkpoint 1: make the representation fact explicit

An ordinary `ColumnViewImpl::array` is conservatively nullable even when its current validity
buffer happens to contain no null bits. `try_non_null_array` inspects that physical fact and returns
a view whose `Nullability` is `NonNull`. Constants with a value are non-null; typed-null and
Indexed views remain nullable.

Do not add a second array representation. The non-null view is a checked promise about the same
fixed-width buffers, so an empty all-valid array is also a valid dense input.

Run the first checkpoint again. Its two tests pin both the nullability classification and the
checked recovery boundary.

## Checkpoint 2: select a dense loop once

Copy the second stage:

```console
cargo x copy-test --chapter 7 --checkpoint 2
cargo test -p type-exercise-starter-expr chapter_7 --locked
```

Implement `PrimitiveBinaryExpression<F>` for a typed `BinaryScalarFunction`. Validate arity,
physical types, and lengths before choosing a path. Then classify each input once:

- a non-null `i32` array supplies a direct values slice;
- a non-null `i32` constant supplies one copied scalar; and
- every other representation delegates to `evaluate_binary`.

Expose the selected shape as `PrimitiveLoop` so the supplied test can prove the choice rather than
infer it from timing. Each specialized loop returns `Vec<i32>` and is source-equivalent to the
simple scalar traversal:

```rust,ignore
for row in 0..len {
    output.push(function(left[row], right[row]));
}
```

The actual array/constant shapes vary only in how each operand is loaded. No operation match,
nullability branch, erased scalar conversion, or builder validity update belongs in that loop.

## Checkpoint 3: preserve the fallback contract

Copy the completed Chapter 7 test:

```console
cargo x copy-test --chapter 7 --checkpoint 3
cargo test -p type-exercise-starter-expr chapter_7 --locked
cargo check -p type-exercise-starter-core --locked
```

The seven focused tests now prove all four dense shapes, general fallback for nullable and Indexed
views, wrapping `i32` addition, metadata nullability, and unchanged arity/type/length errors. The
core-only check proves the reusable machinery has no dependency on `I32Add` or any facade module.

Install `cargo-expand` if needed, then inspect the selected implementation:

```console
cargo expand -p type-exercise-starter-expr --lib arithmetic
```

Locate the concrete `I32Add` path and follow its call into the core evaluator. The expanded facade
must not contain an operation-specific batch loop. In the core dense helper, confirm that the hot
loop performs only typed loads, the preselected scalar call, and pushes. This is a source-level
ownership check, not a promise that LLVM will vectorize every target in the same way.

## Why the fallback stays

The strict no-branch loop is impossible for the general representation. `ColumnView::get` must
distinguish null rows, and `ArrayBuilder::push(Option<_>)` must update validity. Moving those facts
into generated code would hide the branches rather than remove them. Selecting the dense path once
per batch gives the fast case its honest simple loop while retaining correct null and dictionary
semantics everywhere else.

Next: [Chapter 8 adds three-valued Boolean logic](./chapter-8-runtime-erasure.md).

{{#include copyright.md}}
