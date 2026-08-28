{{#include wip-banner.md}}

# Chapter 6: Make Arity Systematic

Chapter 5 chose one typed binary kernel before each batch, then reused the checked binary shell for
every row. Its validator already accepts a slice of expected input types, but it remains private
inside the operator module. A new three-input function would still be easy to implement as another
special case, repeating the arity, physical-type, length, null, output, and error work that Chapters
4 and 5 separated from the scalar operation.

This chapter makes that boundary explicit. You will publish the shared validator already used by
the unary and binary shells, add a checked ternary shell with the same batch contract, and execute
`clamp(value, lower, upper)` through it. The result is not a generic expression framework. It is
one more concrete arity that shows which parts of evaluation vary with the number of inputs and
which parts stay unchanged.

## What is in the starter

Begin from your completed Chapter 5 workspace. In `src/operators.rs`, the checked unary and binary
traits and expressions are working code. Both expressions already call
`validate_expression_inputs`, but that helper is private. The file ends with comment shells for
the Day 6 additions:

- the public shared validator;
- `CheckedTernaryScalarFunction` and `TernaryExpression`;
- the physical numeric `neg` and `clamp` builders; and
- the exact re-export to add in `src/lib.rs`.

You own those additions and the `ScalarError::InvalidClampBounds` variant used by `clamp`. Keep the
existing unary and binary row loops intact. Logical function registration waits until Chapter 9,
and concrete four- or five-input builtins are not part of this chapter.

Chapter 6 has three cumulative supplied checkpoints. Copy the first one before editing:

```console
cargo x copy-test --chapter 6 --checkpoint 1
cargo test -p type-exercise-starter chapter_6 --locked
```

The focused run should fail because the Day 5 validator is still private. Do not edit the copied
test. Checkpoints 1 and 2 deliberately avoid importing later constructors; Checkpoint 3 copies the
completed Chapter 6 test.

## Checkpoint 1: share validation across arities

Open `src/operators.rs`. `validate_expression_inputs` already checks a batch before either the
unary or binary row loop allocates an output or calls a scalar function. Make that helper public
without changing its order:

1. compare the actual and expected arities;
2. compare physical types in input order;
3. compare every later input length with input zero; and then
4. allow row evaluation to begin.

That precedence is observable when more than one fact is wrong. A type error in a later column
must win over an earlier length mismatch because all physical types are checked before any
length. The helper returns the batch length only after every check passes.

Its expected types are a slice rather than a two-element array. The supplied test uses that fact
directly: four valid two-row inputs return `Ok(2)`, while wrong arity, a later wrong type, and a
later wrong length produce their existing `ExpressionError` categories. This does not require a
four-input expression. It proves that validation itself is not binary-specific.

Publish only the validator from `src/lib.rs`; `ExpressionError` is already public:

```rust,ignore
pub use operators::validate_expression_inputs;
```

Run the same checkpoint again:

```console
cargo x copy-test --chapter 6 --checkpoint 1
cargo test -p type-exercise-starter chapter_6 --locked
```

Passing this checkpoint means the shared boundary works for an arbitrary expected-type slice.
The ternary API is still absent.

## Checkpoint 2: add one typed ternary loop

Now add `CheckedTernaryScalarFunction` beside the unary and binary hooks. Give it associated
`First`, `Second`, `Third`, and `Output` scalar families. Its scalar method receives the three
associated borrowed types and returns either the associated owned output or `ScalarError`.

`TernaryExpression<F>` stores the function name, the three physical input types derived from those
associated families, and `F`. Its constructor should derive the types rather than accept a second
runtime description that could disagree with the trait.

The evaluation method follows the boundary you already built:

1. call `validate_expression_inputs` before allocating output;
2. convert the three erased columns to `ColumnView<F::First>`, `ColumnView<F::Second>`, and
   `ColumnView<F::Third>` once;
3. read all three positions for each row;
4. call the scalar function only when all three are non-null; and
5. append the associated output, a strict null, or the existing row-carrying scalar error.

Add `ScalarError::InvalidClampBounds` in `src/expression.rs`. The supplied witness uses it in a
small mixed-family clamp with `i16`, `i32`, and `i64` inputs and an `i64` output. It also verifies
that a null in any input skips the scalar call and that invalid bounds preserve the function name
and failing row.

Copy and run the cumulative second checkpoint:

```console
cargo x copy-test --chapter 6 --checkpoint 2
cargo test -p type-exercise-starter chapter_6 --locked
```

Passing now means the direct typed ternary shell is complete. Numeric `clamp` selection is still
missing.

## Checkpoint 3: select a real ternary kernel once

Finish `src/operators.rs` with the crate-private physical builders named by the starter:
`build_numeric_neg_expression` and `build_numeric_clamp_expression`. They receive already-selected
physical families, just as Chapter 5's binary builder does. Chapter 9 will place logical name
binding in front of them.

Numeric negation reuses `UnaryExpression`. For signed integers, apply the standard `Neg` trait to
`std::num::Wrapping<T>` and recover `.0`; negating `MIN` then has the same wrapping result in debug
and release builds. Floating-point negation uses the ordinary standard operation. Strict null
handling still belongs to the unary shell.

Clamp is the observable three-input path. A private `NumericClamp<A, B, C, O>` implements the new
checked ternary hook. Promote the value, lower bound, and upper bound to `O` inside that typed
scalar call. Bounds are valid only when `lower.partial_cmp(&upper)` is `Less` or `Equal`. A lower
bound greater than the upper bound, or an unordered floating-point comparison involving `NaN`,
returns `InvalidClampBounds`.

Choose the whole-batch kernel from the exact `(value, lower, upper, output)` physical tuple once,
before evaluation begins. That kernel enters `TernaryExpression`, converts the three columns once,
and runs its single typed row loop. Do not materialize three promoted arrays or match erased scalar
variants inside each row.

The legal tuple comes from applying Chapter 5's lossless promotion table twice: first to `(value,
lower)`, then to that result and `upper`. The second result is the output family. For example,
`(i16, i32, i64) -> i64` and `(i32, f32, i16) -> f64` are legal. A tuple that needs a missing
promotion, such as one mixing `i64` with a floating-point family, never reaches this physical
builder.

Copy the final checkpoint and run the completed contract:

```console
cargo x copy-test --chapter 6 --checkpoint 3
cargo test -p type-exercise-starter chapter_6 --locked
cargo test -p type-exercise-starter --lib --locked
```

The focused cases now cover generic validation beyond ternary arity, the direct mixed-family
ternary witness, strict null propagation, row-carrying invalid-bound errors, wrapping numeric
negation, every legal two-step clamp promotion tuple, and rejection of greater or unordered
bounds. The cumulative library run keeps the Chapter 1–5 type, array, column-view, and expression
contracts in the same learner workspace.

## Read the shared boundary

Unary, binary, and ternary expressions have different typed scalar hooks, but the surrounding
batch contract is the same. The shared validator answers whether row evaluation may begin. Each
shell then recovers typed borrowed columns once, applies its arity-specific strict-null rule, and
builds the associated output array. The physical clamp selector chooses one concrete instantiation
before that work starts.

Before continuing, make sure you can explain these boundaries in your own words:

1. Why must every physical type be checked before the first length mismatch is reported?
2. Why does an expected-type slice prove more than a validator hard-coded for three inputs?
3. Why does `TernaryExpression<F>` derive its three physical types from `F` instead of accepting
   them separately?
4. Why is an unordered `NaN` bound an error while a null input skips the clamp scalar call?

You now have a real ternary expression without weakening the typed row boundary or duplicating the
batch checks. Chapter 7 will apply the same separation to three-valued Boolean logic, where nulls
are part of the operator's truth table rather than always strict.

Next: [Chapter 7 adds three-valued Boolean logic with SQL null semantics](./chapter-7-boolean-logic.md).

{{#include copyright.md}}
