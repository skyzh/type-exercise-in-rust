{{#include wip-banner.md}}

# Checkpoint 6: Separate Binary Semantics

Checkpoint 5 can lift a strict, infallible scalar function over nullable columns. That contract is
not enough for every binary operation. Checked division can fail on a non-null row, while SQL
Boolean `AND` and `OR` sometimes produce a known answer even when one input is null.

This checkpoint gives each case its own core boundary. Begin from completed Checkpoint 5, then copy
the cumulative tests:

```console
cargo x copy-test --chapter 6
cargo test -p type-exercise-starter-supplied-tests chapter_6 --locked
```

The focused run should fail only because these three public functions are missing from
`core/src/expression.rs`:

- `auto_vectorize_primitive_i32`;
- `try_evaluate_binary`; and
- `evaluate_nullable_binary`.

Implement them in that file. Keep any raw column helper private.

## A raw path for total Int32 operations

Some `(i32, i32) -> i32` operations are strict, total, and infallible. Wrapping addition is one example:
every pair of non-null inputs has exactly one output, and null in either input produces null.
`auto_vectorize_primitive_i32` may combine the value buffers and validity bitmaps directly for the
Array/Constant cross-product:

```rust,ignore
pub fn auto_vectorize_primitive_i32<F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: Fn(i32, i32) -> i32;
```

Validate both Int32 inputs and their lengths before evaluating. If either view is Indexed, use the
existing typed auto-vectorizer instead. That preserves one well-tested path for indirection. The
result must have the same visible values and nulls either way; callers do not observe which path
ran.

This raw contract is deliberately narrow. A callback that can fail or assigns meaning to null
cannot safely use it.

## Checked division: strict but fallible

`try_evaluate_binary` lifts a fallible scalar function:

```rust,ignore
pub fn try_evaluate_binary<L, R, O, F, E>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function_name: &str,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: Fn(L, R) -> Result<O, E>,
    E: std::fmt::Display;
```

Validate type and length before calling `function`. For each row:

1. if either input is null, append null without calling the scalar function;
2. otherwise call the function once;
3. append its value on success; or
4. stop at the first failure and add the function name and row to the returned error.

A checked division callback is now an ordinary scalar function:

```rust,ignore
let output = try_evaluate_binary::<i32, i32, i32, _, _>(
    left,
    right,
    "checked_divide",
    |left, right| {
        if right == 0 { Err("division by zero") } else { Ok(left / right) }
    },
)?;
```

A null numerator with a zero denominator remains null because strict lifting does not call the
callback for that row. A later non-null division by zero is the first reported error.

## Three-valued Boolean logic: null is an input

Strict lifting cannot express SQL Boolean logic. `false AND null` is known to be false, and
`true OR null` is known to be true. The nullable-aware evaluator therefore passes both optional
values to the callback on every row:

```rust,ignore
pub fn evaluate_nullable_binary<L, R, O, F>(
    left: ColumnViewImpl<'_>,
    right: ColumnViewImpl<'_>,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: FnMut(Option<L>, Option<R>) -> anyhow::Result<Option<O>>;
```

For `AND`, return `Some(false)` when either input is `Some(false)`, `Some(true)` when both are true,
and `None` otherwise. For `OR`, return `Some(true)` when either input is true, `Some(false)` when
both are false, and `None` otherwise. Validation still happens once before the first callback.

Run the focused and cumulative checks:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_6 --locked
cargo test -p type-exercise-starter-supplied-tests --locked
```

Passing means the three policies agree on validation and owned output construction while keeping
their different callback and null contracts. Concrete arithmetic and Boolean expression builders
arrive later; this checkpoint stops at reusable core lifting.

[Checkpoint 7 constructs concrete arithmetic expressions](./chapter-7-boolean-logic.md).

{{#include copyright.md}}
