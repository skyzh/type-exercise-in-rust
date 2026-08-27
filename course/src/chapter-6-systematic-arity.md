{{#include wip-banner.md}}

# Chapter 6: Make Arity Systematic

Chapter 5 selected one typed binary kernel before each batch. Now you will reuse the unary and
binary adapters, publish their shared validator, and add a real three-input path. `clamp(value,
lower, upper)` will prove that validation, strict null handling, and output construction are modeled
by arity rather than by “binary.”

**Prerequisites:** Chapter 5 and slice pattern matching.

**By the end of this chapter, you will:**

- share arity, type, and length validation across expression shells;
- reuse the unary and binary adapters and implement a ternary adapter; and
- execute `clamp(value, lower, upper)` through a real typed ternary shell.

```console
cargo x copy-test --chapter 6 --checkpoint 1
cargo test -p type-exercise-starter chapter_6 --locked
```

The first run should fail because the Day 5 validator is still private. Each checkpoint below has a
cumulative supplied test, so later constructors are not imported before you implement them.

## Check the boundary before rows

`validate_expression_inputs` owns this precedence:

1. arity;
2. physical types in input order;
3. every length against input zero; then
4. row evaluation.

It returns the batch length only after all checks pass. The helper accepts any expected-type slice,
so the tests exercise four- and five-input validation even though the course does not add concrete
four- or five-input builtins. Four valid two-row inputs return `Ok(2)`; wrong arity, a later wrong
type, and a later wrong length report their distinct existing `ExpressionError` categories.

## Checkpoint 1: share validation

- **Target:** `type-exercise-starter/src/operators.rs::validate_expression_inputs`.
- **Change:** make the existing Day 5 helper public. Keep its arity-before-type-before-length order
  and reuse the `ExpressionError` contract that Chapter 4 already established.
- **Preserve:** no output allocation or scalar call before validation completes.
- **Run:** copy and run the Checkpoint 1 test.
- **Passing means:** the generic validator accepts four valid inputs as `Ok(2)` and reports the
  named arity, type, and length categories in the promised precedence.

Publish only the validator; `ExpressionError` is already public:

```rust,ignore
pub use operators::validate_expression_inputs;
```

## Checkpoint 2: add the ternary shell

- **Target:** `type-exercise-starter/src/operators.rs::{CheckedTernaryScalarFunction, TernaryExpression}`.
- **Change:** give the checked hook associated `First`, `Second`, and `Third` scalar families;
  derive the expected physical inputs from them, convert each column to its typed view once, apply
  the same strict row contract to three inputs, and build the associated output array.
- **Change:** add the Day 6-only `ScalarError::InvalidClampBounds` category used by the supplied
  custom ternary witness and the final `clamp` function.
- **Preserve:** a null in any position skips the scalar function; scalar failure carries its row.
- **Run:** `cargo x copy-test --chapter 6 --checkpoint 2`, then the focused test.
- **Passing means:** a direct `TernaryExpression` with `i16`, `i32`, and `i64` inputs produces
  `i64`, preserves array/constant/null positions, and attaches the function name and row to a
  checked scalar failure.

## Checkpoint 3: make ternary behavior observable

- **Target:** `type-exercise-starter/src/operators.rs::build_numeric_clamp_expression` and
  `build_numeric_neg_expression`.
- **Change:** construct physical `neg` and `clamp` shells for their already-selected numeric
  families. Signed negation must use `Wrapping`, so negating `MIN` is identical in debug and
  release builds. Clamp bounds are valid only when `lower.partial_cmp(&upper)` is `Less` or
  `Equal`; `lower > upper` and unordered floating-point bounds involving `NaN` both fail.
- **Preserve:** physical selection happens once before the batch. Store one whole-batch kernel
  selected from the exact `(value, lower, upper, output)` tuple. Its typed
  `NumericClamp<A, B, C, O>` scalar hook promotes each value into `O` inside the single ternary row
  loop; do not materialize promoted arrays first.
- **Run:** `cargo x copy-test --chapter 6 --checkpoint 3`, then focused and cumulative tests.
- **Passing means:** `neg` exercises the strict unary path and `clamp` executes through the real
  three-input batch path, not only in a source-level smoke test.

Clamp uses Chapter 5's lossless promotion table twice: first promote `(value, lower)`, then promote
that result with `upper`. The second result is the output family. Thus
`(i16, i32, i64) -> i64` and `(i32, f32, i16) -> f64` are legal, while any tuple that requires a
missing promotion—such as mixing `i64` with a floating-point family—is not. Chapter 9 will bind
logical calls; this chapter receives an already-legal physical tuple and chooses its one typed
kernel.

## Required and extension work

Unary, binary, real ternary, and generic validation for longer slices are required. Logical
registration waits until Chapter 9. Concrete four- and five-input builtins are extensions. Keep
their scalar hooks typed and any runtime erasure at the whole-batch boundary.

```console
cargo test -p type-exercise-starter chapter_6 --locked
cargo test -p type-exercise-starter --lib --locked
```


Next: [Chapter 7 adds three-valued Boolean logic with SQL null semantics](./chapter-7-boolean-logic.md).

{{#include copyright.md}}
