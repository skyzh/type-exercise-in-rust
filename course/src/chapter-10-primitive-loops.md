# Checkpoint 10: Add One-Level Lists and Batch Async

The final checkpoint adds two boundaries without changing the scalar functions you already built:

1. a checked, nullable, one-level `List` value family; and
2. a future that defers one complete, already-bound batch expression.

Start from your completed Checkpoint 9 workspace. Copy the cumulative public contract without
opening its source first:

```console
cargo x copy-test --chapter 10
cargo test -p type-exercise-starter-supplied-tests chapter_10 --locked
```

That focused test should initially fail only because the new List and async names do not exist.
Keep every earlier checkpoint green as you work.

## Stage 1: represent nullable one-level Lists

Add `PhysicalType::List(Box<PhysicalType>)`, then extend the erased array, scalar-reference, and
column-view families with checked List variants. Implement public equivalents of `ListScalar`,
`ListScalarRef`, `ListArray`, and `ListColumnView` at the Checkpoint 10 comments in the starter.

A List has two independent layers of nullability. The outer validity says whether the row itself
is null. For a present row, the child array may still contain null elements. Empty and all-null
Lists therefore cannot infer their child type: callers always provide an explicit non-List child
`PhysicalType`, including the complete Decimal descriptor when the child is Decimal.

For a raw List array, validate all of these invariants before publishing the value:

- nested List and Map children are rejected;
- the child array has exactly the declared physical type;
- there is one more offset than outer rows, the first offset is zero, offsets never decrease,
  and the final offset equals the child length;
- a null row repeats its preceding offset; and
- row and slice ranges are checked.

A failed constructor, append, or slice must not expose partial state. Array, Constant, and Indexed
column views must yield equivalent safe borrowed List rows through the usual checked access path.
Do not prescribe a field layout in your public API; preserve these observable semantics instead.

Exercise the boundary directly before moving on:

```rust,ignore
let child = StringArray::from_slice(&[Some("left"), None, Some("right")]);
let row = ListScalar::try_new(ArrayImpl::String(child))?;
let lists = ListArray::try_from_rows(
    PhysicalType::String,
    [Some(row.as_list_ref()), None],
)?;

assert_eq!(lists.get(0)?.unwrap().len(), 3);
assert!(lists.get(0)?.unwrap().get(1)?.is_none()); // null child element
assert!(lists.get(1)?.is_none());                  // null List row
assert_eq!(lists.slice(0, 1)?.len(), 1);
# Ok::<(), ListError>(())
```

## Stage 2: defer one already-bound batch

Keep the Checkpoint 9 binder and all synchronous expression behavior unchanged. Add public
equivalents of these four boundaries in the expression core:

First strengthen the existing trait declaration to `pub trait Expression: Send + Sync`. This is
learner-owned work, not a bound supplied elsewhere: the borrowing future is `Send`, so both the
compiler-known expression reference and the expression erased inside `Box<dyn Expression>` must
be safe to share across threads.

- `BatchFuture<'a>`: a `Send` future borrowing the expression and input views;
- `evaluate_static`: a compiler-known async entry point;
- dyn-compatible `AsyncExpression`; and
- `AsyncExpressionAdapter` over the existing `Box<dyn Expression>`.

Creating the future must not evaluate anything. When driven, it evaluates the child exactly once
for the complete batch and preserves the same owned array or error as synchronous evaluation.
The future may borrow the expression and views and makes no `Unpin` promise.

Bind the logical call you built in Checkpoint 9, then run the same complete batch through all three
paths:

```rust,ignore
let left: ArrayImpl = I16Array::from_slice(&[Some(2), None, Some(-4)]).into();
let inputs = [
    ColumnViewImpl::array(&left),
    ColumnViewImpl::constant(ScalarRefImpl::Int32(5), 3),
];

let bound = bind_logical_call(LogicalCall::new(
    "+",
    [DataType::SmallInt, DataType::Integer],
))?;

let sync = bound.evaluate(&inputs)?;
let static_async = evaluate_static(bound.physical_expression(), &inputs).await?;
let erased = AsyncExpressionAdapter::new(bound.into_physical_expression());
let erased_async = erased.evaluate_async(&inputs).await?;

assert_eq!(sync, static_async);
assert_eq!(sync, erased_async);
# Ok::<(), anyhow::Error>(())
```

This is an ownership and API boundary, not a scheduler. Do not add an executor, runtime, I/O,
retries, cancellation policy, locks, per-row futures, recursive Lists, Maps, or new scalar
semantics. Your implementation only defers one existing whole-batch computation.

Finish by running the cumulative contract:

```console
cargo test -p type-exercise-starter-supplied-tests --locked
```
