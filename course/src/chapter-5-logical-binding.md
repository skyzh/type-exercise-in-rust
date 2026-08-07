# Chapter 5: Bind Logical Expressions

Chapter 4 can construct a concrete physical expression such as `i32_add` from a runtime name.
Query planning starts one layer earlier. A parsed call contains a logical name and logical types:

```text
"+" + [Integer, Integer]  ->  binder  ->  i32_add
```

The binder rejects invalid signatures before execution and records the logical result type. The
physical expression remains responsible for column encodings, batch lengths, nulls, and the row
loop.

## Starting Point and Result

Continue from your Chapter 4 implementation and copy the next supplied contract:

```console
cargo x copy-test --chapter 5
cargo test -p type-exercise-starter chapter_5 --locked
```

After this chapter, add these public pieces:

- `DataType`, the planner-visible logical type;
- `BindError`, which distinguishes name, signature, catalog, and metadata failures;
- `BoundExpression`, a checked logical signature paired with one physical expression; and
- `FunctionRegistry`, which binds logical binary names to factories.

The builtin registry supports integer `+` and string `concat`. Keep their selection logic at
planning time. Do not add casts, per-row dispatch, or a primitive fast path yet.

## Keep Logical and Physical Types Separate

The physical layer has two storage and scalar families: `Int32` and `String`. A planner needs more
meaning than storage alone carries:

```rust,ignore
pub enum DataType {
    Integer,
    Varchar,
    Char { width: u16 },
}
```

`Varchar` and `Char { width }` are distinct logical types even though both use the existing
`PhysicalType::String` family. Implement a total mapping:

| Logical type | Physical type |
| --- | --- |
| `Integer` | `Int32` |
| `Varchar` | `String` |
| `Char { .. }` | `String` |

The mapping does not implement SQL `CHAR` padding or truncation. By the time this physical
expression framework receives a string value, any such normalization belongs to an earlier
logical or storage boundary. The width remains available to the planner instead of disappearing
into `PhysicalType::String`.

## Bind a Name and Signature Once

Use a registry of logical binary factories. Its public operation accepts the name and two logical
types, then returns one checked expression:

```rust,ignore
registry.bind_binary("+", DataType::Integer, DataType::Integer)
registry.bind_binary(
    "concat",
    DataType::Char { width: 4 },
    DataType::Varchar,
)
```

The builtin signatures are:

| Logical name | Accepted inputs | Logical output | Physical expression |
| --- | --- | --- | --- |
| `+` | `Integer, Integer` | `Integer` | `i32_add` |
| `concat` | any `Varchar`/`Char` pair | `Varchar` | `string_concat` |

Selection happens once during planning. A successful `BoundExpression::evaluate` delegates
directly to the already-selected `dyn Expression`; it must not inspect logical types or look up a
name for every batch or row.

Allow callers to register another logical name with its own binary factory. Store the factory as
a boxed `Fn(DataType, DataType) -> Result<BoundExpression, BindError>`. Chapter 7 will revisit the
extra bounds needed to share registries across threads, so do not add `Send + Sync` preemptively.

## Distinguish Name and Signature Failures

These calls fail for different reasons:

```text
missing(Integer, Integer)  -> unknown logical name
+(Varchar, Integer)        -> known name, unsupported arguments
```

Report them as separate `BindError` variants. An unknown name tells the caller that registry
lookup failed. Unsupported arguments tell it that lookup succeeded but no overload accepts that
logical signature. Preserve the original logical types in the second error; reducing both to
physical types would lose the distinction between `Varchar` and `Char`.

Also keep a dedicated error for a logical factory that names a physical expression absent from
the Chapter 4 catalog. That failure identifies registry/catalog drift rather than a bad user call.

## Validate the Logical-to-Physical Boundary

A factory can select the wrong catalog entry while still returning a valid `Box<dyn Expression>`.
For example, it could claim an `Integer, Integer -> Integer` logical signature and attach
`string_concat`. Catch that mismatch when constructing `BoundExpression`:

1. Map both logical input types to their expected physical types.
2. Read the expression's actual input metadata.
3. Map the logical output to its expected physical type.
4. Read the expression's actual output metadata.
5. Return `PhysicalSignatureMismatch` if either side differs.

Store the original logical input and output types after validation. This check is intentionally at
the planning boundary: a valid bound plan does not need to repeat it for each batch.

The check is not a replacement for Chapter 4 execution validation. Runtime inputs can still have
the wrong arity or physical type, or unequal lengths. `BoundExpression::evaluate` must preserve
those existing `ExpressionError` results by delegating without translating them.

## Follow One Call Through the Layers

Binding and executing integer addition now crosses four explicit boundaries:

```text
logical call:         +(Integer, Integer)
binding result:       Integer, Integer -> Integer using i32_add
runtime inputs:       [ColumnViewImpl, ColumnViewImpl]
typed execution:      BinaryExpression<I32Add> -> evaluate_binary -> I32Array
```

For string concatenation, `Char { width: 4 }` and `Varchar` remain visible on the bound plan while
both runtime columns use the `String` family. Dictionary, constant, array, and null representations
continue to work because binding does not materialize or rewrite any `ColumnViewImpl`.

## Keep Error Ownership Layered

Each boundary still reports only what it can establish:

| Failure | Owning boundary | Error/result |
| --- | --- | --- |
| unknown logical name | function registry | `BindError::UnknownFunction` |
| unsupported logical pair | logical factory | `BindError::UnsupportedArguments` |
| missing physical name | logical factory/catalog bridge | `BindError::MissingPhysicalExpression` |
| logical/physical metadata drift | bound-expression constructor | `BindError::PhysicalSignatureMismatch` |
| wrong runtime arity or type | erased/typed expression | existing `ExpressionError` |
| unequal runtime lengths | typed evaluator | existing `ExpressionError` |
| null row | typed row loop | null output, not an error |

This separation makes a bind error a planning failure and an expression error an execution-input
failure.

## Implementation Checkpoints

1. Define `DataType` and its mapping to `PhysicalType`.
2. Define comparable `BindError` variants and useful `Display` messages.
3. Make `BoundExpression::new` validate logical and physical metadata.
4. Add a registry of logical binary factories.
5. Register `+` and `concat` with the signatures above.
6. Delegate bound evaluation to the Chapter 4 expression unchanged.
7. Run the focused contract and keep Chapters 1–4 green.

## Review Your Chapter Result

Run:

```console
cargo test -p type-exercise-starter chapter_5 --locked
cargo test -p type-exercise-starter --lib --locked
```

The Chapter 5 contract contains seven tests. They cover integer and string binding, preservation of
distinct `Char`/`Varchar` logical metadata, unknown names, unsupported signatures, a mismatched
physical factory, delegated execution errors, and custom logical registration.

Before continuing, explain:

- why `DataType` cannot be replaced by `PhysicalType` in the binder;
- why a known name with unsupported arguments differs from an unknown name;
- why `BoundExpression` validates physical metadata once but retains logical metadata;
- why runtime arity, type, length, and null behavior remain below the binder; and
- why logical selection does not belong inside the row loop.

Chapter 6 will keep this binding API and specialize the primitive all-valid execution path. The
optimization must preserve every result and error boundary established so far.

{{#include copyright.md}}
