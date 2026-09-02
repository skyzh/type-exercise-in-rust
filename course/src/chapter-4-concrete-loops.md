# Chapter 4: Expose the Cost of Concrete Loops

Chapter 3 gave arrays, constants, typed nulls, and indexed values one borrowed row interface. That
solved a representation problem. It did not yet turn a scalar operation such as `i32 + i32` into a
batch expression.

A batch adapter has more work to do than the addition itself. It must reject the wrong inputs
before indexing, preserve strict nulls, build the correct output family, and stop cleanly if a row
operation fails. This chapter writes that machinery as a fixed-arity whole-batch boundary. The
kernel pointer is erased, but the operation it names is still vectorized: there is no dynamically
dispatched object call for every scalar row.

## What is in the starter

Begin from your completed Chapter 3 workspace. `core/src/column.rs` already provides
`ColumnViewImpl<'a>` and the checked `ColumnView<'a, S>` conversion. The Chapter 4 files contain only
comment shells:

- `core/src/expression.rs` names the first typed binary scalar function, evaluator, fixed-arity
  batch shell, and vectorized kernel pointer;
- `expr/src/numeric.rs` names the concrete `I32Add` scalar operation; and
- `core/src/lib.rs` and `expr/src/lib.rs` keep their respective modules and exports commented out.

Build two pieces:

1. one typed binary scalar operation lifted over nullable borrowed columns; and
2. one batch expression that validates a complete fixed-arity input contract before delegating to
   a monomorphized row loop.

The later comments are boundaries, not implementation work. Leave numeric promotion, ternary
evaluation, the erased `Expression` trait and catalog, primitive fast paths, and asynchronous
adapters for their chapters.

Copy the cumulative supplied test before editing:

```console
cargo x copy-test --chapter 4
cargo test -p type-exercise-starter-supplied-tests chapter_4 --locked
```

The first focused run should fail because the Chapter 4 modules and public items do not exist yet. Do
not edit the copied test.

## Checkpoint 1: keep one row operation small

Open `core/src/expression.rs` and define `BinaryScalarFunction` with three associated scalar families:
`Left`, `Right`, and `Output`. Its method receives the borrowed scalar-reference type for each
input and returns one owned output value.

That signature keeps one row operation independent from the column representation:

```rust,ignore
pub trait BinaryScalarFunction {
    type Left: Scalar;
    type Right: Scalar;
    type Output: Scalar + Copy;

    fn evaluate<'a>(
        &self,
        left: <Self::Left as Scalar>::RefType<'a>,
        right: <Self::Right as Scalar>::RefType<'a>,
    ) -> Self::Output;
}
```

In `expr/src/numeric.rs`, implement `I32Add` first. Use `wrapping_add` explicitly. Ordinary signed addition can panic on
overflow in a debug build and wrap in a release build; a database expression must not change its
result with the compilation profile.

Next implement `evaluate_binary`. It receives two `ColumnViewImpl<'a>` values and one typed scalar
function. The adapter, not the function, owns the batch work:

1. convert each erased input once to `ColumnView<'a, F::Left>` or
   `ColumnView<'a, F::Right>`;
2. reject unequal lengths before reading a row;
3. allocate the builder associated with `F::Output` for that length;
4. for each row, call the scalar function only when both inputs are non-null; and
5. finish the builder and erase the owned output as `ArrayImpl`.

The typed conversions perform the physical-family checks. They also recover the borrowed scalar
shape established in Chapter 1: a mixed-family function can receive `&str` from a string column and
`i32` from a primitive column without allocating either input value. This first adapter deliberately
requires a copyable fixed-width output. A variable-width result cannot be built safely as one
temporary scalar value here; Chapter 10 introduces a transactional string builder at the
whole-batch boundary.

Use `anyhow::Result<ArrayImpl>` for batch failures. Add ordinary context that identifies the input
position for type or length failures and preserves the underlying cause. Enable `expression` in
`core/src/lib.rs` and export the checkpoint's public function and trait. Enable `numeric` in
`expr/src/lib.rs` and export `I32Add`; there is no course-specific runtime error enum to maintain.

The copied Chapter 4 test also imports the Checkpoint 2 shells, so it cannot be green yet. Use an
honest library boundary here:

```console
cargo check -p type-exercise-starter-expr --lib --locked
```

Passing means the first vectorized loop and its public surface compile. The completed focused test
will later exercise arrays, constants, and indexed views; strict nulls; a borrowed mixed-family
function; an output family different from the inputs; explicit wrapping overflow; and type and
length rejection.

## Checkpoint 2: erase one complete batch operation

Now return to `core/src/expression.rs`. Define `BatchExpression<const N: usize>` with a static function name,
an `[PhysicalType; N]` input contract, one output type, and a function pointer for a complete
batch. Name that pointer type `BatchKernel<N>`. Its signature receives the expression metadata and
the borrowed `&[ColumnViewImpl<'_>]`, and returns `anyhow::Result<ArrayImpl>`.

This is the important erasure boundary. A caller may select one monomorphized batch kernel at
runtime, but that kernel converts each erased column to a typed `ColumnView` once and owns the
whole row loop. Do not introduce unary, binary, or ternary checked scalar traits, and do not store
a dynamically dispatched scalar operation inside the loop.

`BatchExpression::new` receives the complete metadata and kernel. Its inherent `evaluate` method
accepts `&[ColumnViewImpl<'_>]`. Before calling the kernel, validate in this order:

1. the input count equals its arity;
2. every input's physical family equals the corresponding expected family; and
3. every input has the same logical length as the first input.

That order is observable. An empty unary input slice is an arity error, not an indexing panic. A
wrong second physical family is rejected before the row loop. A binary length mismatch is rejected
before the selected batch kernel runs.

Write small test kernels for arity one and arity two. Inside each kernel, recover its typed views,
allocate the associated output builder, and make each row follow one strict rule:

```text
any required input is null -> append null; do not perform the operation
all required inputs are set -> perform the typed operation once
typed operation returns Err -> stop; return a batch error and no output array
```

Report the three validation failures with contextual messages that preserve expected and actual
values; type and length failures must also identify the input index. Null and error are different
results: a strict null is a valid row in the output, while an operation error ends evaluation and
later rows must not run. Add the function name and first failing row to that underlying cause. Do
not return a partially built array.

The kernel borrows every input view and returns a new owned array. Do not materialize an input
representation just to simplify the loop.

Export `BatchExpression` and `BatchKernel` through the already enabled `expression` module in
`core/src/lib.rs`. Keep the later `Expression` trait and runtime catalog commented out.

Run the focused contract, then the cumulative learner-library suite:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_4 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
```

The 14 focused cases prove the complete boundary:

- one typed binary evaluator works over array, constant, typed-null, and indexed representations;
- borrowed mixed-family inputs and a copyable fixed-width associated output family work without
  per-representation loops;
- `i32` addition has explicit wrapping behavior;
- fixed-arity batch expressions reject arity, physical-family, and length errors before kernel
  entry;
- strict nulls skip the typed operation and append null; and
- a row error inside a whole-batch kernel stops later rows and returns no partial array.

## Compare the repeated boundary

Compare the arity-one and arity-two kernels you exercised through the same shell:

| Decision | Unary | Binary | Same underlying rule? |
| --- | --- | --- | --- |
| Arity | exactly one input | exactly two inputs | yes |
| Physical types | check one expected family | check two expected families | yes |
| Length | establish one batch length | require both lengths to match | yes |
| Strict null | skip on one null | skip if either input is null | yes |
| Row failure | stop at the failing row | stop at the failing row | yes |
| Output | typed builder in the kernel | typed builder in the kernel | yes |

The shell captures the repeated boundary decisions without erasing individual scalar operations.
Chapter 5 will select generic numeric batch kernels while preserving these rules. Chapter 6 will
make the shared validator public and add vectorized negation and clamp. Runtime trait-object
erasure comes later, after the whole-batch path has concrete behavior to preserve.

Before generalizing the loop, check three distinctions:

1. Why is strict null propagation batch control flow rather than a scalar-operation object?
2. Why must arity, physical types, and lengths be checked before the first row is evaluated?
3. Why is a whole-batch function pointer a different boundary from a dynamically dispatched call
   on every scalar row?

This module ends with the exact work required to lift one scalar operation over a nullable batch.
[Chapter 5 makes numeric operation selection generic](./chapter-5-generic-arithmetic.md) without
moving dynamic dispatch into the row loop.

{{#include copyright.md}}
