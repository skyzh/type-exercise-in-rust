# Chapter 4: Expose the Cost of Concrete Loops

Write one strict unary loop and one strict binary loop before trying to generalize them. Their
duplication is the evidence that later abstraction must remove.

**Prerequisites:** Chapter 3, associated output types, and ordinary `Result` handling.

**By the end of this chapter, you will:**

- separate scalar work from batch validation and output construction;
- preserve strict nulls without invoking the scalar function; and
- identify the repeated decisions in unary and binary adapters.

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

The concrete shells use the batch `Expression` trait statically in this chapter. Runtime names,
trait objects, and catalog selection wait until Chapter 7.

## Checkpoint 1: define scalar work

- **Target:** `type-exercise-starter/src/expression.rs::{BinaryScalarFunction, I32Add, evaluate_binary}` for the initial
  binary path; `type-exercise-starter/src/operators.rs::{CheckedUnaryScalarFunction, CheckedBinaryScalarFunction}` for
  the two checked arities.
- **Change:** keep scalar functions responsible only for non-null scalar inputs and their operation.
- **Preserve:** integer addition uses explicit wrapping semantics.
- **Run:** the Chapter 4 focused test.
- **Passing means:** the same function works over arrays, constants, and dictionaries.

## Checkpoint 2: write the arity-specific loops

- **Target:** `type-exercise-starter/src/operators.rs::{UnaryExpression, CheckedBinaryExpression}` and their
  `Expression::evaluate` implementations.
- **Change:** implement the five repeated decisions once for each arity without a shared arity
  generator yet.
- **Preserve:** arity, type, and length errors precede row access; a null row never calls the
  scalar function.
- **Run:** focused and cumulative tests.
- **Passing means:** unary and binary behavior agree on validation, nulls, and output ownership.

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

Next: [Chapter 5 makes numeric operation selection generic](./chapter-5-generic-arithmetic.md).

{{#include copyright.md}}
