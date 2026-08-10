# Chapter 4: Expose the Cost of Concrete Loops

Write one strict typed binary evaluator and one strict unary and binary shell before trying to
generalize them. The duplication is the evidence that later abstraction must remove.

**Prerequisites:** Chapter 3, associated output types, and ordinary `Result` handling.

**By the end of this chapter, you will:**

- separate scalar work from batch validation and output construction;
- preserve strict nulls without invoking the scalar function; and
- identify the repeated decisions every unary and binary adapter must honor.

```console
cargo x copy-test --chapter 4
cargo test -p type-exercise-starter chapter_4 --locked
```

The first run should fail on the missing scalar-function traits, adapters, and batch evaluator.

## Mark the repeated decisions

Both arities must:

1. check input count before indexing;
2. check physical types before reading rows;
3. check every input length before allocating output;
4. skip the scalar function if any strict input is null; and
5. build the associated output array or return the first row error without partial output.

The number of inputs changes, but those rules do not.

The checked shells expose a strict concrete evaluation now; shared validation and the structured
error shape arrive in Chapter 6, and the erased `Expression` boundary plus the runtime catalog
come in later chapters. Choose a readable error type for both paths now — the supplied tests only
require an `Err` result, not a particular error representation.

Wire the new modules like Chapter 3 did for `column`:

```rust,ignore
mod expression;
mod operators;
pub use expression::{BinaryScalarFunction, I32Add, ScalarError, evaluate_binary};
pub use operators::{
    CheckedBinaryExpression, CheckedBinaryScalarFunction, CheckedUnaryScalarFunction,
    UnaryExpression,
};
```

Keep the public items in `expression.rs`/`operators.rs`; this module wiring lets the copied test
import them from the starter crate root.

## Checkpoint 1: define scalar work

- **Target:** `type-exercise-starter/src/expression.rs::{BinaryScalarFunction, I32Add, ScalarError, evaluate_binary}`.
- **Change:** keep scalar functions responsible only for non-null scalar inputs and their operation;
  return a readable error type of your choice on mismatched length or physical type.
- **Preserve:** integer addition uses explicit wrapping semantics.
- **Run:** the Chapter 4 focused test.
- **Passing means:** the same function works over arrays, constants, and Indexed views, and
  rejects wrong input shapes without evaluating rows.

## Checkpoint 2: write the arity-specific loops

- **Target:** `type-exercise-starter/src/operators.rs::{CheckedUnaryScalarFunction, CheckedBinaryScalarFunction, UnaryExpression, CheckedBinaryExpression}`.
- **Change:** implement the five repeated decisions once for each arity. The shells own
  `new(name, input_types, function)` and an inherent `evaluate(&[ColumnViewImpl<'_>])` returning
  `Result<ArrayImpl, _>` with your readable error type; no shared arity generator and no
  `Expression` implementation yet.
- **Preserve:** arity, type, and length errors precede row access; a null row never calls the
  scalar function; a scalar `Err` becomes a batch `Err`, never a null row.
- **Run:** the Chapter 4 focused test and the cumulative suite.
- **Passing means:** unary and binary behavior agree on arity, type, length, nulls, and output
  ownership.

The repeated code is intentional. You should now be able to point to what Chapter 5 generalizes
and what Chapter 6 makes systematic.

## Required and extension work

One checked unary shell and one checked binary shell are required. More operations, runtime
catalogs, and ternary evaluation are later work. Do not create a generic N-ary vector of erased
values as an extension; it would throw away the typed family you just built.

```console
cargo test -p type-exercise-starter chapter_4 --locked
cargo test -p type-exercise-starter --lib --locked
```


{{#include copyright.md}}
