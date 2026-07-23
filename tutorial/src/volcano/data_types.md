# Bind Logical Data Types

An unbound function call contains a name and logical input types. The binder chooses one supported
signature or returns an error before execution.

For comparisons, the course records triples:

```text
{ left input, right input, common comparison type }
{ int16,     int32,       int32 }
{ int32,     float32,     float64 }
{ varchar,   fwchar,      varchar }
```

The third type is not necessarily an output type. A comparison casts both inputs to the common type
and returns Boolean. Numeric addition uses a similar table but returns the promoted common type.

## Why a Table Instead of One Clever Trait?

Rust knows which conversions implement `From`, but SQL promotion policy is a database decision.
The language cannot decide whether `i32 + f32` should return `f32`, return `f64`, or be rejected.
Encoding the policy as explicit data makes it reviewable.

The final binder exposes:

```rust
pub fn bind_binary_expression(
    function: ExpressionFunc,
    left: DataType,
    right: DataType,
) -> Result<BoundExpression, BindError>;
```

`BoundExpression` stores:

- the checked logical input types;
- the logical output type; and
- a concrete typed kernel behind `Box<dyn Expression>`.

For `SmallInt + Integer`, binding selects
`BinaryExpression<i16, i32, i32, _>` and reports `DataType::Integer`. For
`Integer contains Varchar`, it returns `BindError::UnsupportedArguments`.

## Registration Keeps Custom Functions Local

`FunctionRegistry` maps a name to a planning-time factory:

```rust
let registry = FunctionRegistry::with_builtins();
let expression = registry.bind_binary(
    "contains",
    DataType::Varchar,
    DataType::Char { width: 10 },
)?;
```

A custom function can register a custom factory. It does not need to join the numeric promotion
matrix or modify the vectorizer. Its factory checks the logical types and returns a typed expression
using the common runtime interface.

This is the most important reframing in the course:

- `+`, `<=`, `>=`, `=`, and `!=` benefit from generated combinations;
- `contains` is one explicit string implementation;
- future JSON, date/time, list, geospatial, and regular-expression functions should normally remain
  explicit implementations for their own data types.

## Binder Invariants

After binding succeeds:

1. the expression arity is fixed;
2. each input has an expected physical type;
3. the output logical type is known; and
4. the runtime kernel's generic parameters agree with those types.

`BoundExpression::eval` verifies that incoming column views have the promised physical types. The
generated kernel can then downcast through the common framework. Function authors never write the
downcast.

## Chapter Checkpoint

Explain why these calls have different outcomes:

```text
bind("+", SmallInt, Integer)       -> Integer
bind("contains", Varchar, Char)   -> Boolean
bind("contains", Integer, Varchar)-> error
```

Next, we [draw the boundary](./framework.md) that the vectorized runtime will implement.

{{#include ../copyright.md}}
