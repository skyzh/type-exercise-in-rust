# Chapter 6: Make Arity Systematic

Two correct loops can still be a special case. A real three-input function proves that validation,
strict null handling, and output construction are modeled by arity rather than by “binary.”

**Prerequisites:** Chapter 5 and slice pattern matching.

**By the end of this chapter, you will:**

- share arity, type, and length validation across expression shells;
- implement unary, binary, and ternary adapters; and
- execute `clamp(value, lower, upper)` through a real typed ternary shell.

```console
cargo x copy-test --chapter 6
cargo test -p type-exercise-starter chapter_6 --locked
```

The first run should fail on shared validation or the unary and ternary physical paths.

## Check the boundary before rows

`validate_expression_inputs` owns this precedence:

1. arity;
2. physical types in input order;
3. every length against input zero; then
4. row evaluation.

It returns the batch length only after all checks pass. The helper accepts any expected-type slice,
so the tests exercise four- and five-input validation even though the course does not add concrete
four- or five-input builtins.

## Checkpoint 1: share validation

- **Target:** `type-exercise-starter/src/operators.rs::validate_expression_inputs` and
  `type-exercise-starter/src/expression.rs::ExpressionError`.
- **Change:** report arity before indexing, type before length, and the exact mismatching input index.
- **Preserve:** no output allocation or scalar call before validation completes.
- **Run:** the Chapter 6 focused test.
- **Passing means:** two-, four-, and five-input slices fail closed with the same error vocabulary.

## Checkpoint 2: add the ternary shell

- **Target:** `type-exercise-starter/src/operators.rs::{CheckedTernaryScalarFunction, TernaryExpression}`.
- **Change:** apply the same strict row contract to three inputs and build the associated output
  array.
- **Preserve:** a null in any position skips the scalar function; scalar failure carries its row.
- **Run:** the focused test.
- **Passing means:** mixed array/constant/null inputs retain position and null semantics.

## Checkpoint 3: make ternary behavior observable

- **Target:** `type-exercise-starter/src/operators.rs::build_numeric_clamp_expression`.
- **Change:** construct a physical `clamp` shell for the already-selected common numeric family and
  reject `lower > upper`.
- **Preserve:** physical selection happens once before the batch; the typed shell remains the only
  row loop.
- **Run:** focused and cumulative tests.
- **Passing means:** `clamp` executes through the real three-input batch path, not only in a
  source-level smoke test.

## Required and extension work

Unary, binary, real ternary, and generic validation for longer slices are required. Logical
registration waits until Chapter 9. Concrete four- and five-input builtins are extensions. If you
generate boilerplate, use a source-controlled declarative macro.

```console
cargo test -p type-exercise-starter chapter_6 --locked
cargo test -p type-exercise-starter --lib --locked
```


{{#include copyright.md}}
