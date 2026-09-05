# Checkpoint 3: Build Shared Typed Evaluation

You now have owned nullable arrays and borrowed Array, Constant, Null, and Indexed views. This
checkpoint turns them into one complete evaluation path: validate a batch once, read typed rows
through `ColumnView::get`, call one scalar function when its inputs are present, and append a newly
owned output array. Later optimizations will still fall back to this path.

Begin from your completed Checkpoint 2 workspace. Copy the cumulative tests, then run only the new
Chapter 3 cases:

```console
cargo x copy-test --chapter 3
cargo test -p type-exercise-starter-supplied-tests chapter_3 --locked
```

The focused test should fail because the shared evaluators and the three numeric facade functions
do not exist yet. The inherited Chapter 1 and 2 APIs should still compile; keep the copied tests
unchanged.

## Validate before traversing rows

Enable the existing `expression` module and export it from
`type-exercise-starter/core/src/lib.rs`. In `core/src/expression.rs`, implement:

```rust,ignore
pub fn validate_expression_inputs(
    inputs: &[ColumnViewImpl<'_>],
    expected_types: &[PhysicalType],
) -> anyhow::Result<usize>
```

Reject an arity mismatch first. Then compare each input's physical type with its expected type and
check that every input has the same length as the first. Return that common length; an empty input
list has length zero.

After validation, the row loop can rely on two facts: each typed view has the requested family,
and every input can be read at each output row.

## Lift scalar functions through typed views

Implement three public evaluators in the same core module:

```rust,ignore
evaluate_unary::<I, O, _>(input, scalar_function)
evaluate_binary::<L, R, O, _>(left, right, scalar_function)
evaluate_ternary::<A, B, C, O, _>(first, second, third, scalar_function)
```

Each evaluator follows one sequence:

1. call `validate_expression_inputs` with the scalar families' `PHYSICAL_TYPE` values;
2. convert every erased input to `ColumnView<S>` once;
3. allocate `<O as Scalar>::ArrayType::Builder` for the validated row count;
4. read each row with typed `get` and call the scalar function only when every input is non-null;
5. append the resulting value or null, finish the builder, and erase the owned array.

Use `Option::map` for unary input and `Option::zip` for binary and ternary inputs. That makes strict
null propagation part of the shared traversal: a null input produces a null output without calling
the scalar function.

Let `ColumnView::get` hide the Array, Constant, and Indexed variants. It is the
representation-generic path that remains correct when later checkpoints place faster loops in
front of it, and Indexed inputs can continue to use it unchanged.

## Choose numeric meaning in the facade

Enable `numeric` in `type-exercise-starter/expr/src/lib.rs`. Core owns validation, traversal, null
propagation, and output construction. The expr facade chooses concrete types and one scalar
operation.

Expose these exact functions from `expr/src/numeric.rs`:

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

`add_i16_i32` instantiates `i16 + i32 -> i32`, converting the left scalar with `i32::from`.
`negate_i32` uses wrapping negation. `clamp_i32` instantiates the ternary evaluator with
`i32::clamp`. Each function delegates the complete batch to one core evaluator. The facade chooses
the numeric meaning without owning a row loop or knowing how the columns are represented.

## Run the cumulative contract

Run the focused Chapter 3 cases, then every copied test:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_3 --locked
cargo test -p type-exercise-starter-supplied-tests --locked
```

The cumulative run should pass twelve tests: five from Checkpoint 1, four from Checkpoint 2, and
three from Checkpoint 3. The new cases cover the public numeric facade, mixed numeric types,
Array/Constant/Indexed inputs, strict null propagation, owned output, and arity/type/length
validation.

You can also run the completed snapshot independently:

```console
cargo test -p type-exercise-checkpoint-03-supplied-tests --locked
cargo test -p type-exercise-checkpoint-03-expr --lib --locked
cargo check -p type-exercise-checkpoint-03-core --locked
```

You are done when all three scalar arities share one typed-`get` path and the facade contains only
the concrete numeric choices. The next checkpoint tackles the different publication rule needed
by variable-width output.

{{#include copyright.md}}
