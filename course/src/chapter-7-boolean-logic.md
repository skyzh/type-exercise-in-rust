# Checkpoint 7: Erase Whole-Batch Expressions

Checkpoint 6 can evaluate complete batches, but callers still choose each evaluator directly.
Build one value that carries a batch function together with its physical contract, then make that
value usable through one runtime-erased interface.

Begin from completed Checkpoint 6 and copy the cumulative tests:

```console
cargo x copy-test --chapter 7
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
```

The focused run should fail only because `BatchKernel`, `BatchExpression`, and `Expression` are
missing from `core/src/expression.rs`.

## Give a complete batch one function type

Start with a function pointer that accepts any lifetime used by the borrowed input views:

```rust,ignore
pub type BatchKernel =
    for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
```

The higher-ranked lifetime means the function works with the batch borrowed by each call. A plain
function pointer also keeps this checkpoint focused on evaluation: it cannot capture a catalog,
binder, or per-row state.

For example, an already-earned evaluator becomes a kernel without rebuilding its loop:

```rust,ignore
fn i32_add(inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl> {
    auto_vectorize_binary::<i32, i32, i32, _>(
        inputs[0].clone(),
        inputs[1].clone(),
        i32::wrapping_add,
    )
}
```

The expression shell will validate the inputs before this function can index them.

## Keep fixed arity while it is known

Add `BatchExpression<const N: usize>`. It owns:

- a `&'static str` name;
- exactly `[PhysicalType; N]` input types;
- one output `PhysicalType`; and
- one `BatchKernel`.

Provide `new`, `name`, `input_types`, `output_type`, and `evaluate`. Its direct API keeps the input
arity in the type and makes the physical contract inspectable:

```rust,ignore
let add = BatchExpression::new(
    "i32_add",
    [PhysicalType::Int32, PhysicalType::Int32],
    PhysicalType::Int32,
    i32_add,
);

assert_eq!(add.input_types().len(), 2);
let output = add.evaluate(&[left, right])?;
```

`evaluate` has two validation boundaries. First call `validate_expression_inputs` with the stored
input types. Only after arity, physical types, and row counts pass may the kernel run. Then reject
a returned array whose physical type differs from `output_type` or whose length differs from the
validated input length. A successful call returns the owned array unchanged.

## Erase the shell, not each row

Different fixed arities cannot share one collection directly. Define a dyn-compatible
`Expression` trait with `name`, `input_types`, `arity`, `output_type`, and `evaluate`. `arity` can
default to the length of `input_types`.

Implement the trait for every `BatchExpression<N>`. The erased path delegates to the same checked
whole-batch evaluation:

```rust,ignore
let expression: Box<dyn Expression> = Box::new(add);
assert_eq!(expression.arity(), 2);
let output = expression.evaluate(&[left, right])?;
```

The dynamic choice happens once for the complete batch. Rows still run inside the existing typed
evaluators, so this boundary does not introduce a virtual call or erased scalar value per row.

Run the focused and cumulative checks:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
cargo test -p type-exercise-starter-supplied-tests --locked
```

Passing means metadata and direct evaluation survive erasure, invalid inputs never reach the
kernel, and invalid kernel outputs are rejected. Concrete expression factories and a physical
function catalog arrive later; this checkpoint stops at the reusable whole-batch boundary.

{{#include copyright.md}}
