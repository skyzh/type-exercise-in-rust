# Checkpoint 9: Bind Logical Calls to Physical Expressions

Checkpoint 8 accepts a physical function and exact physical input types. A query planner begins one
level earlier, with a logical function name and logical input types such as `SmallInt`, `Integer`,
`Char`, or `Varchar`. In this checkpoint, you will connect those two worlds with one binding layer.

Begin from completed Checkpoint 8 and copy the cumulative tests:

```console
cargo x copy-test --chapter 9
cargo test -p type-exercise-starter-supplied-tests chapter_9 --locked
```

The focused run should fail only because the Checkpoint 9 logical-call, bound-expression, and binder
names are missing.

## Describe one logical call

In `expr/src/binder.rs`, implement this public surface:

```rust,ignore
pub struct LogicalCall { /* logical name and input DataTypes */ }

impl LogicalCall {
    pub fn new(
        name: impl Into<String>,
        input_types: impl IntoIterator<Item = DataType>,
    ) -> Self;
    pub fn name(&self) -> &str;
    pub fn input_types(&self) -> &[DataType];
}

pub enum BindError { /* unknown, wrong arity, unsupported, metadata mismatch */ }

pub struct BoundExpression { /* logical contract plus Box<dyn Expression> */ }

pub fn bind_logical_call(call: LogicalCall) -> Result<BoundExpression, BindError>;
```

`BoundExpression` should expose the logical call and output type, a borrowed view of the selected
physical expression, a way to take its `Box<dyn Expression>`, and an `evaluate` method that delegates
the entire batch. Its constructor must reject logical metadata whose physical input or output types
disagree with the expression it wraps.

Enable the binder module from `expr/src/lib.rs`. Logical binding belongs in the facade. Core
continues to own arrays, logical and physical representations, validation, generic traversal,
writers, and the erased batch boundary.

## Resolve only the maintained overloads

Map these logical names to the physical catalog identifiers already earned in Checkpoint 8:

| Logical names | Accepted logical inputs | Logical output |
| --- | --- | --- |
| `+`, `-`, `*`, `/` | two losslessly compatible numeric types | their promoted numeric type |
| `neg` | one supported numeric type | the input type |
| `clamp` | three pairwise-promotable numeric types | the final promoted type |
| `<`, `<=`, `>`, `>=` | compatible numeric types, or two string types | `Boolean` |
| `=`, `!=` | the comparison cases above, or two Booleans | `Boolean` |
| `boolean_and`, `boolean_or` | two Booleans | `Boolean` |
| `boolean_not` | one Boolean | `Boolean` |
| `concat` | any `Char`/`Varchar` pair | `Varchar` |
| `contains` | any `Char`/`Varchar` pair | `Boolean` |

Use the same lossless numeric policy as the physical catalog. `SmallInt` widens to every maintained
numeric family. `Integer` combines with `BigInt` or `Double`; `Integer` plus `Real` produces
`Double`. `Real` combines with `Double`. Reject `BigInt` with either floating family, Decimal
overloads not represented by the physical catalog, and every other unsupported combination.

`Char` and `Varchar` are distinct logical types but both map to physical `String`. That mapping is
why a mixed `Char`/`Varchar` `concat` call selects the existing `StringConcat` expression without a
new row loop or a runtime cast.

Check arity before overload resolution. Reject unknown names and unsupported argument types rather
than guessing. Each accepted call must select exactly one `PhysicalFunction`; then call
`build_physical_expression` with the inputs' physical types and verify the returned metadata before
publishing the bound result.

## Run the complete logical-to-physical loop

The caller now starts with a logical schema, binds once, inspects the public contract, and evaluates
the returned erased expression:

```rust,ignore
let call = LogicalCall::new(
    "+",
    [DataType::SmallInt, DataType::Integer],
);
let bound = bind_logical_call(call)?;

assert_eq!(bound.output_type(), &DataType::Integer);
assert_eq!(
    bound.physical_expression().input_types(),
    &[PhysicalType::Int16, PhysicalType::Int32],
);

let expression: Box<dyn Expression> = bound.into_physical_expression();
let output = expression.evaluate(&[left, right])?;
```

Binding performs logical overload and coercion selection once. Execution remains physical: the
same whole-batch expression validates the actual columns and delegates to its specialized kernel.
Planner trees and casts are outside this course boundary; List storage and async evaluation arrive
in the final checkpoint.

Run the focused and cumulative checks:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_9 --locked
cargo test -p type-exercise-starter-supplied-tests --locked
```

The tests cover exact and widened numeric calls, mixed logical strings, Boolean and ternary binding,
representative rejection paths, checked bound metadata, and evaluation through the returned erased
expression. The result is a complete synchronous path from a logical call to an owned array.

{{#include copyright.md}}
