# Chapter 9: Bind and Coerce Logical Calls

Physical kernels exist, but a parsed call contains logical types and a name. The binder must choose
one kernel, apply only approved widening promotion, and reject unsupported signatures before a
batch runs.

**Prerequisites:** Chapters 2, 5, 7, and 8; `HashMap`; closures; logical versus physical types.

**By the end of this chapter, you will:**

- bind functions of any arity through slice-based metadata;
- keep logical metadata consistent with the chosen physical expression; and
- support correct numeric comparisons, string comparisons, `contains`, and existing `concat`.

```console
cargo x copy-test --chapter 9
cargo test -p type-exercise-starter chapter_9 --locked
```

The first run should fail on slice-based binding, comparison semantics, or `contains`.

## Separate four kinds of conversion

- **Numeric promotion** converts values to a planner-selected common type.
- **Erased downcast** checks whether a runtime enum contains the requested physical family.
- **Trait-object downcast** recovers one concrete expression type through `Any`.
- **Lifetime shortening** reborrows a value for a shorter valid lifetime.

Only the first is logical coercion. Do not call all four “casts” or restore an obsolete GAT upcast
helper.

## Checkpoint 1: generalize the registry

- **Target:** `type-exercise-starter/src/binder.rs::{BindError, BoundExpression, FunctionRegistry::register,
  register_unary, register_binary, register_ternary, bind}`.
- **Change:** store logical inputs as a slice/boxed slice and check requested arity before a factory
  indexes it.
- **Preserve:** unknown name, wrong arity, unsupported arguments, missing physical expression, and
  metadata mismatch remain distinct errors.
- **Run:** the Chapter 9 focused test.
- **Passing means:** `neg`, arithmetic, `clamp`, and custom slice factories share one planning
  boundary across unary, binary, and ternary arities.

`BoundExpression::new` maps logical inputs and output to physical types and compares them with the
selected expression's metadata. A valid bound expression records that proof once; evaluation then
delegates without rebinding each batch.

## Checkpoint 2: bind comparisons and strings

- **Target:** `type-exercise-starter/src/operators.rs::{ComparisonOperator}` and binder factories registered by
  `FunctionRegistry::with_builtins`.
- **Change:** support `<`, `<=`, `>`, `>=`, `=`, `!=`, `contains`, and `concat` for their approved
  logical signatures.
- **Preserve:** names match behavior. Any ordered float comparison with NaN is false; equality is
  false and inequality is true. Null input produces null before comparison.
- **Run:** focused and cumulative tests.
- **Passing means:** equal operands distinguish strict/inclusive operators and NaN never panics.

String `Char` and `Varchar` both use physical `String`, but their logical metadata stays distinct.
This course does not enforce `Char` width.

Wire the binder into the starter crate root like the earlier chapters:

```rust,ignore
mod binder;
pub use binder::{BindError, BoundExpression, FunctionRegistry};
```

## Checkpoint 3: bind three-valued Boolean functions

- **Target:** `type-exercise-starter/src/binder.rs::bind_boolean` and the builtin registry entries
  for `boolean_and`, `boolean_or`, and `boolean_not`.
- **Change:** register the Day 7 expressions through the same slice registry: `boolean_and` and
  `boolean_or` take two `Boolean` inputs, `boolean_not` takes one; the bound output is `Boolean`.
- **Preserve:** arity is checked before a factory indexes its slice; unsupported signatures are
  bind errors; evaluation keeps the SQL three-valued semantics from Day 7.
- **Run:** the focused and cumulative tests.
- **Passing means:** one-input `boolean_not` binds (never rejected by a two-arity signature) and
  bound Boolean evaluation matches the Day 7 truth table.

## Checkpoint 4: keep promotion lossless

- **Target:** `type-exercise-starter/src/promotion.rs::promote_numeric` and its callers in
  `type-exercise-starter/src/binder.rs::{bind_arithmetic, bind_comparison}`.
- **Change:** apply the same approved common type in both paths and both operand orders.
- **Preserve:** unsupported or precision-losing pairs are bind errors; no silent narrowing.
- **Run:** the full Chapter 9 contract.
- **Passing means:** logical output metadata agrees with the chosen physical output.

## Required and extension work

Slice-based binding, lossless promotion, six comparisons, `contains`, and `concat` are required.
Narrowing casts, parsing casts, SQL-complete coercion, Decimal arithmetic, and overload selection
from untyped `NULL` are extensions that need separate semantics.

```console
cargo test -p type-exercise-starter chapter_9 --locked
cargo test -p type-exercise-starter --lib --locked
```


{{#include copyright.md}}
