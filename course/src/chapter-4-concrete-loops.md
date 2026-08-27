{{#include wip-banner.md}}

# Chapter 4: Expose the Cost of Concrete Loops

Chapter 3 gave arrays, constants, typed nulls, and indexed values one borrowed row interface. That
solved a representation problem. It did not yet turn a scalar operation such as `i32 + i32` into a
batch expression.

A batch adapter has more work to do than the addition itself. It must reject the wrong inputs
before indexing, preserve strict nulls, build the correct output family, and stop cleanly if a row
operation fails. This chapter writes that machinery for one unary and one binary shape. The two
loops will look repetitive on purpose: you need to see the stable batch contract before Chapters 5
and 6 generalize it.

## What is in the starter

Begin from your completed Chapter 3 workspace. `src/column.rs` already provides
`ColumnViewImpl<'a>` and the checked `ColumnView<'a, S>` conversion. The Day 4 files contain only
comment shells:

- `src/expression.rs` names the first typed binary scalar function and evaluator;
- `src/operators.rs` names checked unary and binary scalar hooks and their concrete adapters; and
- `src/lib.rs` keeps both modules and their exports commented out.

You own two additions in this chapter:

1. one typed binary scalar operation lifted over nullable borrowed columns; and
2. one checked unary adapter plus one checked binary adapter that make the repeated batch decisions
   visible.

The later comments are boundaries, not implementation work. Leave numeric promotion, ternary
evaluation, the erased `Expression` trait and catalog, primitive fast paths, and asynchronous
adapters for their chapters.

Copy the cumulative supplied test before editing:

```console
cargo x copy-test --chapter 4
cargo test -p type-exercise-starter chapter_4 --locked
```

The first focused run should fail because the Day 4 modules and public items do not exist yet. Do
not edit the copied test.

## Checkpoint 1: keep one row operation small

Open `src/expression.rs` and define `BinaryScalarFunction` with three associated scalar families:
`Left`, `Right`, and `Output`. Its method receives the borrowed scalar-reference type for each
input and returns one owned output value.

That signature keeps one row operation independent from the column representation:

```rust,ignore
pub trait BinaryScalarFunction {
    type Left: Scalar;
    type Right: Scalar;
    type Output: Scalar;

    fn evaluate<'a>(
        &self,
        left: <Self::Left as Scalar>::RefType<'a>,
        right: <Self::Right as Scalar>::RefType<'a>,
    ) -> Self::Output;
}
```

Implement `I32Add` first. Use `wrapping_add` explicitly. Ordinary signed addition can panic on
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
`i32` from a primitive column without allocating either input value. The output family is
independent of both inputs; an `i32, i32 -> String` function must build a `StringArray`.

Choose a readable batch error for failed type or length validation. The course contract checks the
behavior, not a particular public enum name, field layout, or display sentence. Enable
`expression` in `src/lib.rs` and export the checkpoint's public function, trait, `I32Add`, and the
error type you chose.

The copied Chapter 4 test also imports the Checkpoint 2 shells, so it cannot be green yet. Use an
honest library boundary here:

```console
cargo check -p type-exercise-starter --lib --locked
```

Passing means the first vectorized loop and its public surface compile. The completed focused test
will later exercise arrays, constants, and indexed views; strict nulls; a borrowed mixed-family
function; an output family different from the inputs; explicit wrapping overflow; and type and
length rejection.

## Checkpoint 2: put two concrete arities side by side

Now open `src/operators.rs`. Define `CheckedUnaryScalarFunction` and
`CheckedBinaryScalarFunction`. Give the unary hook an associated `Input` family and the binary hook
associated `Left` and `Right` families, in addition to their associated owned output family. Their
methods receive those families' borrowed scalar-reference types and may return `ScalarError`.

The batch adapters derive their expected `PhysicalType` values from those associated input
families. After validating the whole batch, each adapter converts every erased column view once to
the corresponding typed `ColumnView`. The row loop therefore passes `&str`, `i32`, or another
declared borrowed type directly to the checked hook; no checked scalar hook accepts or matches on
`ScalarRefImpl`.

Give both adapters a `new` constructor containing only a static function name and the function
value. The constructor derives its fixed-size expected input-type array from the function's
associated input families, so a caller cannot provide contradictory runtime metadata. Their
inherent `evaluate` methods accept `&[ColumnViewImpl<'_>]` and return an owned `ArrayImpl` or a
readable batch error.

Before allocating an output or indexing `inputs[0]`, each adapter must validate in this order:

1. the input count equals its arity;
2. every input's physical family equals the corresponding expected family; and
3. every input has the same logical length as the first input.

That order is observable. An empty unary input slice is an arity error, not an indexing panic. A
wrong second physical family is rejected before the row loop. A binary length mismatch is rejected
before any scalar call.

After validation, each row follows one strict rule:

```text
any required input is null  -> append null; do not call the scalar function
all required inputs are set -> call the scalar function once
scalar function returns Err -> stop; return a batch error and no output array
```

Null and error are different results. A strict null is a valid row in the output. A scalar error
ends evaluation; it must not be converted into a null row, and later rows must not run. Include the
function name and row in your error if that helps you diagnose the failure, but the supplied test
does not freeze a public error representation.

Both adapters build `F::Output::ArrayType` through its associated builder. They borrow every input
view and return a new owned array. Do not materialize an input representation just to simplify the
loop.

Enable `operators` in `src/lib.rs`. Export the two checked scalar traits, `UnaryExpression`, and
`CheckedBinaryExpression`; also export `ScalarError` from `expression`. Keep the later
`Expression` trait and runtime catalog commented out.

Run the focused contract, then the cumulative learner-library suite:

```console
cargo test -p type-exercise-starter chapter_4 --locked
cargo test -p type-exercise-starter --lib --locked
```

The 13 focused cases prove the complete boundary:

- one typed binary evaluator works over array, constant, typed-null, and indexed representations;
- borrowed mixed-family inputs and an independent associated output family work without
  per-representation loops;
- `i32` addition has explicit wrapping behavior;
- unary and binary adapters reject arity, physical-family, and length errors before row access;
- strict nulls skip the scalar hook and append null; and
- unary or binary scalar failure stops the batch before later rows and returns no partial array.

## Read the duplication as evidence

Compare the concrete unary and binary adapters you just wrote:

| Decision | Unary | Binary | Same underlying rule? |
| --- | --- | --- | --- |
| Arity | exactly one input | exactly two inputs | yes |
| Physical types | check one expected family | check two expected families | yes |
| Length | establish one batch length | require both lengths to match | yes |
| Strict null | skip on one null | skip if either input is null | yes |
| Scalar failure | stop at the failing row | stop at the failing row | yes |
| Output | associated builder | associated builder | yes |

The repeated code is useful because it identifies an abstraction boundary from working cases.
Chapter 5 will make numeric operation selection generic without replacing these batch rules.
Chapter 6 will share validation across arities and add a checked ternary loop. Runtime erasure comes
later, after the typed and checked paths have concrete behavior to preserve.

Before continuing, make sure you can explain three distinctions in your own words:

1. Why is strict null propagation batch control flow rather than an error from the scalar
   function?
2. Why must arity, physical types, and lengths be checked before the first row is evaluated?
3. What did writing unary and binary loops teach you that a generic N-ary loop written first would
   have hidden?

You can now point to the exact work required to lift one scalar operation over a nullable batch.

Next: [Chapter 5 makes numeric operation selection generic](./chapter-5-generic-arithmetic.md).

{{#include copyright.md}}
