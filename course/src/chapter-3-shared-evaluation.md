{{#include wip-banner.md}}

# Chapter 3: Build Shared Typed Evaluation

You now have nullable arrays and one lazy view over Array, Constant, Null, and Indexed inputs. Build
the evaluator that every later optimization must preserve: validate once, read typed rows with
`ColumnView::get`, run one scalar callback, propagate nulls, and append the output.

Copy the cumulative contract:

```console
cargo x copy-test --chapter 3
cargo test -p type-exercise-starter-supplied-tests chapter_3 --locked
```

## Put traversal in core

Open `core/src/expression.rs`. Implement `validate_expression_inputs` before any row loop. It
checks input types and equal row counts and returns the batch length.

Then implement these three public fallbacks:

```rust,ignore
evaluate_unary::<I, O, _>(input, scalar_function)
evaluate_binary::<L, R, O, _>(left, right, scalar_function)
evaluate_ternary::<A, B, C, O, _>(first, second, third, scalar_function)
```

Each function converts the erased views to typed `ColumnView<S>` values once. Inside the loop,
combine the nullable inputs and call the scalar function only when every input row is non-null.
Build `O::ArrayType` through its associated builder.

Do not inspect Array, Constant, or Indexed variants in these functions. The typed `.get` path is
the permanent representation-generic fallback. Later chapters add optional fast paths in front of
it; Indexed inputs will continue to use it.

## Instantiate operations in expr

Enable `expr/src/numeric.rs`. The facade owns scalar meaning and concrete type choices, while core
owns batch traversal. Instantiate a mixed `i16 + i32 -> i32` binary operation, Int32 negation, and
Int32 clamp by calling the corresponding core evaluator. There must be no `for row in 0..` loop in
the expr crate.

Expose these exact adapters from the expr facade:

```rust,ignore
pub fn add_i16_i32(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
) -> anyhow::Result<ArrayImpl>

pub fn negate_i32(input: ColumnViewImpl<'_>) -> anyhow::Result<ArrayImpl>

pub fn clamp_i32(
    value: ColumnViewImpl<'_>,
    lower: ColumnViewImpl<'_>,
    upper: ColumnViewImpl<'_>,
) -> anyhow::Result<ArrayImpl>
```

Each adapter supplies only its scalar closure and type parameters. The cumulative test calls this
public facade, so leaving `numeric` disabled or putting the operations only in core cannot complete
the checkpoint.

This separation is deliberate:

```text
core: validate + traverse + propagate nulls + build output
expr: choose L/R/O + define one scalar operation
Rust: monomorphize that concrete instantiation
```

Run the focused and cumulative checkpoint gates:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_3 --locked
cargo test -p type-exercise-checkpoint-03-supplied-tests --locked
cargo test -p type-exercise-checkpoint-03-expr --lib --locked
cargo check -p type-exercise-checkpoint-03-core --locked
```

The checkpoint is complete when unary, binary, and ternary operations agree across array,
constant, null, and indexed inputs; mixed numeric types produce the chosen output family; and type
or length mismatches fail before evaluation.

Chapter 4 will add variable-width output and a common batch shell without changing this fallback.

{{#include copyright.md}}
