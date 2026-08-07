# Chapter 3: Vectorize a Scalar Function

Chapter 2 made arrays, constants, and dictionaries look like the same nullable rows. This chapter
uses that boundary to lift one scalar operation over a batch:

```text
left:   [10, null, 30]
right:  [ 2,    2,  2]
result: [12, null, 32]
```

The scalar operation sees two `i32` values. The vectorized loop owns length checks, physical-type
checks, null propagation, output allocation, and erased output conversion.

## Starting Point and Result

Continue from your Chapter 2 implementation and copy the next contract:

```console
cargo x copy-test --chapter 3
cargo test -p type-exercise-starter chapter_3 --locked
```

After this chapter, your API has four new pieces:

- `ExpressionError`, which reports checked type and length failures;
- `BinaryScalarFunction`, which names the left, right, and output scalar families;
- `I32Add`, the first typed scalar function; and
- `evaluate_binary`, which evaluates that function over two `ColumnViewImpl` inputs.

Keep the implementation generic even though this chapter supplies only integer addition. Runtime
arity and function-name dispatch belong to Chapter 4.

## Separate Scalar and Batch Responsibilities

The function trait expresses only the non-null scalar rule:

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

`I32Add` connects all three associated types to `i32`. Define overflow deliberately rather than
letting debug and release builds disagree; the course implementation uses `wrapping_add`.

The batch evaluator performs this sequence once per call:

1. Convert both erased views into the scalar families required by `F`.
2. Reject unequal logical lengths.
3. Allocate one output builder with the final row capacity.
4. For each row, call `F` only when both inputs are non-null.
5. Push a null otherwise, finish the typed array, and erase it to `ArrayImpl`.

Do not convert every row through `ScalarImpl`. The view conversion already selected the typed
family, so the hot loop should carry `S::RefType<'a>` values directly.

## Preserve Strict Null Propagation

For a strict binary function, either null input produces a null result:

| Left | Right | Call scalar function? | Output |
| --- | --- | --- | --- |
| value | value | yes | scalar result |
| null | value | no | null |
| value | null | no | null |
| null | null | no | null |

This rule applies equally to null array slots, null constants, null dictionary keys, and dictionary
keys that select null values. `ColumnView` already unifies those cases; the expression loop should
not rediscover their encodings.

## Fail Before Partial Output

Two errors belong before the row loop:

- a view with the wrong physical type returns `ExpressionError::TypeMismatch`; and
- unequal logical lengths return `ExpressionError::InputLengthMismatch`.

Checking first prevents an invalid batch from producing a partial output. Empty inputs of equal
length are valid and return an empty output array.

## Implementation Checkpoints

1. Add a comparable `ExpressionError` with type- and length-mismatch variants.
2. Define `BinaryScalarFunction` and implement `I32Add`.
3. Convert both inputs once with `ColumnView::<F::Left/Right>::try_from`.
4. Build the output through `F::Output`'s associated array builder.
5. Run the focused contract and keep Chapters 1–2 green.

## Review Your Chapter Result

Run:

```console
cargo test -p type-exercise-starter chapter_3 --locked
cargo test -p type-exercise-starter --lib --locked
```

The Chapter 3 contract covers array/constant and dictionary/constant evaluation, strict null
propagation, length mismatch, and physical-type mismatch.

Before continuing, explain:

- which checks happen once and which work happens per row;
- why the scalar function receives non-null borrowed values;
- why output construction goes through the associated array builder; and
- why runtime arity and function names do not belong in this typed loop.

Chapter 4 will erase this interface and generate several typed adapters without duplicating the
batch loop.

{{#include copyright.md}}
