# Day 2: Bind Logical Expressions

Consider `smallint + integer`. The executor receives an `Int16` array and an `Int32` array, but it
should not discover the promotion rule in the row loop. Planning should decide that both values can
be converted to `i32`, that the result is an `Integer`, and that one concrete typed kernel will
evaluate the batch.

Today you will move that decision into a planning-time binder.

## Starting Point and Result

Day 1 produced one input interface for arrays, constants, and dictionaries. The starter also
contains generated expression templates: given a scalar function such as
`fn(i16, i32) -> i32`, a template creates the nullable output loop and hides concrete arrays behind
`Expression`.

Before this day, callers construct an expression directly and invalid signatures may panic through
the compatibility helper. After this day:

- `FunctionRegistry::bind_binary` resolves a function name and logical input types;
- `BoundExpression` records the checked input and output types with the selected kernel;
- unsupported signatures return `BindError` before execution;
- numeric addition and comparison use explicit promotion tables; and
- data-type-specific functions register ordinary factories beside generated families.

This day does not optimize the selected loop. Day 3 will measure the general nullable template
before adding a narrow primitive fast path.

## Planning and Execution Are Different Jobs

Logical and physical types answer different questions:

```text
DataType::Char { width: 10 }  -- SQL-visible semantics
PhysicalType::String          -- StringArray / &str representation
```

`Varchar` and `Char` share a physical representation, but their logical types remain available to
the binder. Conversely, `smallint + integer` uses different physical inputs and selects a promoted
output.

Keep the boundary explicit:

```text
planning
  function name + logical input types
          |
          v
  signature validation + promotion
          |
          v
  BoundExpression

execution
  borrowed physical column views
          |
          v
  one selected typed kernel
          |
          v
  output ArrayImpl
```

The binder runs once per expression. View conversion runs once per batch. The scalar function runs
once per all-valid row.

## The Binding Contract

Implement the contract in `expr-common/src/datatype.rs`, `expr-impl/src/registry.rs`, and
`expr-impl/src/lib.rs`.

1. A known function and supported logical signature produce one `BoundExpression`.
2. `BoundExpression::input_types` preserves the exact logical signature; `output_type` reports the
   planner-selected logical result.
3. An unknown name returns `BindError::UnknownFunction`.
4. A known function with unsupported arguments returns `BindError::UnsupportedArguments`.
5. Before evaluation, each input view's `PhysicalType` must match the physical type implied by the
   bound logical input.
6. Numeric promotion is an explicit table. Do not infer it from Rust's available `Into`
   implementations.
7. Strict null propagation remains owned by the generated evaluator: if either input row is null,
   the result row is null and the scalar kernel is not called.
8. The existing `build_binary_expression` API remains as a compatibility adapter. New callers use
   the fallible binder.

The supported type matrix is a course rule. How the registry stores factories is an implementation
choice as long as registration is thread-safe and binding does not execute a batch.

## Work Through a Promotion

For these rows:

```text
left  SmallInt: [1_i16, null, 3_i16]
right Integer:  [10_i32, 20_i32, 30_i32]
```

binding `+` selects `BinaryExpression<i16, i32, i32, _>` and reports `DataType::Integer`. Execution
then produces:

```text
[11_i32, null, 33_i32]
```

The type choice happens before the values are available. The null at row 1 prevents a kernel call;
it does not change the output type.

The arithmetic kernel is intentionally small:

```rust,ignore
pub fn add<I1, I2, O>(left: I1::RefType<'_>, right: I2::RefType<'_>) -> O
where
    I1: Scalar,
    I2: Scalar,
    O: Scalar + Add<Output = O>,
    for<'a> I1::RefType<'a>: Into<O>,
    for<'a> I2::RefType<'a>: Into<O>,
{
    left.into() + right.into()
}
```

Those bounds prove that one selected combination compiles. They do not decide which combinations
the database supports; the promotion table does.

## Generated Families and Custom Functions

Numeric addition and comparisons have a real cross-product of input types. A macro expands the
supported rows into match arms and concrete generated evaluators.

String containment is different. It has one meaningful physical signature:

```rust,ignore
fn str_contains(left: &str, right: &str) -> bool
```

Register it with an explicit factory that accepts `Varchar` and `Char` logical inputs because both
map to `PhysicalType::String`. Do not force the function through the numeric promotion table.

Use the same checklist for a new function:

- Which logical signatures are valid?
- Which physical scalar types execute them?
- What logical type does the function return?
- Does the function belong to a dense generated family or an explicit custom kernel?
- Which invalid signature proves that the binder rejects misuse before execution?

## Implementation Checkpoints

Work in this order:

1. Add `DataType::physical_type`.
2. Define `BindError` and `BoundExpression`, including the physical-input check.
3. Add a thread-safe binary factory type and `FunctionRegistry::{register_binary, bind_binary}`.
4. Bind numeric addition from the explicit promotion matrix.
5. Route comparisons and `contains` through the same fallible boundary.
6. Retain `build_binary_expression` as the legacy adapter.

Keep changes inside `expr-common/src/datatype.rs` and `expr-impl`. Do not add benchmark
dependencies or primitive-only loops yet.

## Verify the Day

Run:

```console
cargo test -p expr-impl --locked
```

The tests should cover:

- `smallint + integer` producing an `Integer` array with strict null propagation;
- a bound `contains` expression reading Day 1 dictionary and constant views;
- rejection of `contains(integer, varchar)`; and
- equality at the `>=` boundary.

Before moving on, explain:

- why `DataType` cannot be replaced by `PhysicalType` during planning;
- where the promotion decision is made;
- which checks occur once per expression, once per batch, and once per row; and
- why a custom string function belongs in the registry but not the numeric matrix.

Next, you will specialize and measure the primitive hot path.

{{#include copyright.md}}
