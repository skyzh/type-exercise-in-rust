{{#include wip-banner.md}}

# Chapter 6: Make Arity Systematic

Chapter 5 chose one typed binary adapter before each batch and already centralized both infallible
and fallible binary traversal in the core package. The facade delegates instead of repeating a row
loop. The remaining boundary is incomplete: validation is still private, and unary and ternary
scalar callbacks do not yet have the same reusable core path.

This chapter moves reusable row machinery into the core package. You will publish the validator and
implement monomorphized evaluators for the taught unary, binary, and ternary callback shapes. A
generated physical adapter then supplies only an ordinary scalar function such as `neg_number(value)`
or `clamp_number(value, lower, upper)`. There is no erased `dyn Fn` and no operation-specific row
loop; infallible and fallible callbacks may use separate shared core helpers because their return
contracts differ.

## What is in the starter

Begin from your completed Chapter 5 workspace. In `expr/src/numeric.rs`, selected arithmetic adapters
already delegate to the core binary helpers. `validate_expression_inputs` is private. The file ends
with comment shells for the Chapter 6 additions:

- the public shared validator;
- shared unary and ternary evaluators alongside the existing binary boundary;
- scalar-only negation and clamp operations plus generated physical selectors;
- the physical numeric `neg` and `clamp` builders; and
- the core visibility change that publishes the shared validator, plus removal of Chapter 5's
  facade-local duplicate so the facade's existing core re-export exposes the shared function.

Implement those additions and the contextual invalid-bound error used by `clamp`. Keep the new unary
and ternary adapters free of operation-specific row loops as you route them through the shared
helpers. Logical function registration waits until Chapter 11, and a fourth arity is not part of
this chapter.

Chapter 6 has three cumulative supplied checkpoints. Copy the first one before editing:

```console
cargo x copy-test --chapter 6 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_6 --locked
```

The focused run should fail because the Chapter 5 core validator is private and the facade-local helper
shadows its name. Do not edit the copied test. Checkpoints 1 and 2 deliberately avoid importing later
constructors; Checkpoint 3 copies the completed Chapter 6 test.

## Checkpoint 1: share validation across arities

Open `core/src/expression.rs`. `validate_expression_inputs` already checks a batch before either the
unary or binary row loop allocates an output or calls a scalar function. Make that helper public
without changing its order:

1. compare the actual and expected arities;
2. compare physical types in input order;
3. compare every later input length with input zero; and then
4. allow row evaluation to begin.

That precedence is observable when more than one fact is wrong. A type error in a later column
must win over an earlier length mismatch because all physical types are checked before any
length. The helper returns the batch length only after every check passes.

Its expected types are a slice rather than a two-element array. Four valid two-row inputs return
`Ok(2)`, while wrong arity, a later wrong type, and a later wrong length return contextual
`anyhow` messages with expected and actual values plus the
input index where applicable. This does not require a four-input expression. It proves that
validation itself is not binary-specific.

Make the validator public in `core/src/expression.rs`. Then remove the duplicate validator from
`expr/src/numeric.rs` and import the core function there instead. The core package already exports
its expression module through `core/src/lib.rs`, and the facade already re-exports that core surface:

```rust,ignore
pub fn validate_expression_inputs(/* existing arguments */) -> anyhow::Result<usize> {
    // existing validation order
}
```

Run the same checkpoint again:

```console
cargo x copy-test --chapter 6 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_6 --locked
```

Passing this checkpoint means the shared boundary works for an arbitrary expected-type slice.
The ternary API is still absent.

## Checkpoint 2: move each callback shape into a shared core loop

Implement shared unary and ternary evaluators and preserve the reusable binary boundary from Chapter
5. Each helper is generic over concrete scalar families and the callback type, so Rust
monomorphizes the call. Each helper validates and recovers typed borrowed columns once, then owns its
reusable row loop in the core package. Start the ternary path with the checkpoint's `(i16, i32,
i64) -> i64` witness.

The selected generated adapter follows the boundary you already built:

1. call `validate_expression_inputs` before allocating output;
2. convert the three erased columns to `ColumnView<i16>`, `ColumnView<i32>`, and `ColumnView<i64>`
   once;
3. read all three positions for each row;
4. call the supplied scalar function only when all three values are present; and
5. append the result, a strict null, or return an error with the function, row, and cause.

Return the cheap concrete cause `Err("invalid clamp bounds")` when the lower and upper values are
reversed or unordered. Keep the fallible ternary helper generic over that error type: successful rows
return `Ok(value)` without dynamic error construction, and the outer row loop creates the contextual
`anyhow` error only after it observes a failure. The supplied witness uses this boundary in a small
mixed-family clamp with `i16`, `i32`, and `i64` inputs and an `i64` output. It also verifies that a null
in any input skips the operation and that invalid bounds preserve the function name and failing row.

Copy and run the cumulative second checkpoint:

```console
cargo x copy-test --chapter 6 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_6 --locked
```

Passing now means the reusable ternary auto-vectorizer works. The full generated `clamp` selector
is still missing.

## Checkpoint 3: author scalar work and generate adapters

Finish `expr/src/numeric.rs` with the public physical builders named by the starter:
`build_numeric_neg_expression` and `build_numeric_clamp_expression`. The separate supplied-test
crate calls them directly. They receive already-selected physical families, just as Chapter 5's
binary builder does. Chapter 11 will place logical name binding in front of them.

`neg_number` owns only one scalar value. For signed integers, apply the standard `Neg`
trait to `std::num::Wrapping<T>` and recover `.0`; negating `MIN` then has the same wrapping result
in debug and release builds. Floating-point negation uses the ordinary standard operation. The
generated adapter calls `evaluate_unary(column, neg_number)` and contains no row loop.

Clamp is the observable three-input path. A generated adapter promotes `A`, `B`, and `C` to `O`
with infallible `TryFrom`, then passes the values to scalar-only `clamp_number`. Bounds are valid
only when `lower.partial_cmp(&upper)` is
`Less` or `Equal`. A lower bound greater than the upper bound, or an unordered floating-point
comparison involving `NaN`, returns the same static invalid-bound cause; the shared outer loop adds
the function name and row only on that failure.

Choose the generated adapter from the exact `(value, lower, upper, output)` physical tuple once,
before evaluation begins. Do not materialize promoted arrays, match erased scalar variants inside
each row, or copy the ternary loop into that adapter.

The legal tuple comes from applying Chapter 5's lossless promotion table twice: first to `(value,
lower)`, then to that result and `upper`. The second result is the output family. For example,
`(i16, i32, i64) -> i64` and `(i32, f32, i16) -> f64` are legal. A tuple that needs a missing
promotion, such as one mixing `i64` with a floating-point family, never reaches this physical
builder.

Copy the final checkpoint and run the completed contract:

```console
cargo x copy-test --chapter 6 --checkpoint 3
cargo test -p type-exercise-starter-supplied-tests chapter_6 --locked
cargo test -p type-exercise-starter-expr --lib --locked
```

The focused cases now cover generic validation beyond ternary arity, the direct mixed-family
ternary witness, strict null propagation, row-carrying invalid-bound errors, wrapping numeric
negation, every legal two-step clamp promotion tuple, and rejection of greater or unordered
bounds. The cumulative library run keeps the Chapter 1–5 type, array, column-view, and expression
contracts in the same learner workspace.

Inspect the facade after macro expansion:

```console
cargo expand -p type-exercise-starter-expr --lib numeric
```

Locate a concrete numeric adapter such as negation or addition. It should convert or select its
typed scalar function, then delegate to `evaluate_unary`, `auto_vectorize_binary`, or
`try_evaluate_ternary` in the core package. It must not contain an operation-specific `for row`
loop. `cargo expand` is an inspection step rather than a build dependency; the ordinary tests
remain the completion gate.

## Trace the shared boundary

Unary, binary, and ternary expressions share core auto-vectorizers by arity and callback/null policy.
The helpers own validation, typed view recovery, null handling, row context, and output construction.
Generated physical adapters own promotion and select the scalar function; scalar functions own only
one value-level operation.

Check the boundary before leaving generic numeric evaluation:

1. Why must every physical type be checked before the first length mismatch is reported?
2. Why does an expected-type slice prove more than a validator hard-coded for three inputs?
3. Why must the clamp selector bind its three physical input types and output type to one batch
   kernel before row evaluation begins?
4. Why is an unordered `NaN` bound an error while a null input skips the clamp scalar call?

The module ends with a real ternary expression and no scalar-operation erasure or duplicated batch
checks. [Chapter 7 adds fixed-width dense paths](./chapter-7-boolean-logic.md) while preserving the
nullable-aware fallback.

{{#include copyright.md}}
