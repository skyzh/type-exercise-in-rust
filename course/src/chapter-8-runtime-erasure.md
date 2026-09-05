# Checkpoint 8: Build the Physical Expression Catalog

Checkpoint 7 erased one already-constructed whole-batch expression. A caller still needs to know
which concrete builder to call. This checkpoint adds a catalog that turns a physical function
identifier plus exact physical input types into `Box<dyn Expression>`.

Begin from completed Checkpoint 7 and copy the cumulative tests:

```console
cargo x copy-test --chapter 8
cargo test -p type-exercise-starter-supplied-tests chapter_8 --locked
```

The focused run should fail only because `try_auto_vectorize_ternary` and the Checkpoint 8
catalog/factory surface are missing.

## Complete the fallible ternary bridge

Checked division already established the strict fallible rule for two inputs. Add its ternary
counterpart in `core/src/expression.rs`:

```rust,ignore
pub fn try_auto_vectorize_ternary<A, B, C, O, F, E>(
    first: ColumnViewImpl<'_>,
    second: ColumnViewImpl<'_>,
    third: ColumnViewImpl<'_>,
    function_name: &str,
    function: F,
) -> anyhow::Result<ArrayImpl>
where
    F: Fn(A, B, C) -> Result<O, E>,
    E: std::fmt::Display;
```

Validate all three physical types and lengths before the first callback. Skip the callback when any
input is null. Stop at the first non-null scalar failure and return an error with useful function
and row context. Specialize Array/Array/Array and send every other shape through the typed fallback,
just as the infallible ternary adapter does. Both routes build a fresh owned output.

This small core-owned bridge lets physical `clamp` report invalid bounds without panicking or
moving a row loop into the facade.

## Give each scalar family concrete builders

Enable the numeric, Boolean, and String facade modules. Each module owns scalar meaning and selects
an existing core evaluator once for a complete batch:

- numeric builders cover losslessly widened `+`, `-`, `*`, `/`, negation, fallible clamp, and six
  comparisons;
- Boolean builders cover three-valued `AND` and `OR`, strict `NOT`, equality, and inequality; and
- String builders cover writer-backed concatenation, containment, and six comparisons.

Choose the physical signature before entering any kernel. Integer overflow uses the course's
wrapping arithmetic rule. Division and clamp use fallible core lifts. String concatenation writes
directly through the consumed `Writer`, so a partially written failing row cannot be published.

Lossless numeric widening is the same for arithmetic, comparisons, and each step of clamp:

- `Int16` widens to any numeric family;
- `Int32` combines with `Int64` or `Float64`;
- `Int32` plus `Float32` produces `Float64`;
- `Float32` combines with `Float64`; and
- `Int64` with either floating family is rejected.

List is not a numeric family here.

## Build one discoverable physical catalog

In `expr/src/catalog.rs`, implement this public surface:

```rust,ignore
pub enum PhysicalFunction { /* numeric, Boolean, and String functions */ }

pub struct PhysicalFunctionEntry {
    pub function: PhysicalFunction,
    pub name: &'static str,
    pub arity: usize,
}

pub const PHYSICAL_FUNCTION_CATALOG: &[PhysicalFunctionEntry];

pub fn find_physical_function(name: &str) -> Option<PhysicalFunction>;

pub fn build_physical_expression(
    function: PhysicalFunction,
    inputs: &[PhysicalType],
) -> anyhow::Result<Box<dyn Expression>>;
```

The catalog metadata is for discovery. Construction is the checked boundary: reject unsupported
arity or physical input types before returning an expression. Numeric construction computes one
lossless common physical type, then instantiates the matching typed builder. Boolean and String
construction accept only their exact physical signatures.

At this boundary, the caller already has physical columns and deliberately chooses a physical
function. Logical names, casts, and overload resolution belong one level earlier and will enter in
Checkpoint 9.

## Use the complete physical loop

Suppose execution already holds an `Int16` column and an `Int32` column. The caller can inspect
those physical types, choose the catalog's numeric-add identifier, and ask for one erased
expression:

```rust,ignore
let function = find_physical_function("numeric_add").expect("catalog entry");
let expression = build_physical_expression(
    function,
    &[PhysicalType::Int16, PhysicalType::Int32],
)?;

assert_eq!(expression.output_type(), PhysicalType::Int32);
let output = expression.evaluate(&[left, right])?;
```

The dynamic choice happens once. The returned expression validates the actual batch and delegates
rows to its already-selected typed kernel.

Run the focused and cumulative checks:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_8 --locked
cargo test -p type-exercise-starter-supplied-tests --locked
```

The tests cover every supported and rejected physical signature plus representative mixed numeric,
fallible clamp, nullable Boolean, and transactional String evaluation through `dyn Expression`.
With physical selection complete, Checkpoint 9 can decide which function a SQL name and logical
schema should mean.

{{#include copyright.md}}
