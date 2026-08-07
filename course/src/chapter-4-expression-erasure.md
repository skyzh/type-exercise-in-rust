# Chapter 4: Erase and Generate Expressions

Chapter 3 can vectorize any typed `BinaryScalarFunction`, but its caller must still know the
function's Rust type:

```rust,ignore
evaluate_binary(&I32Add, left, right)
```

A query executor does not have that type at runtime. It has a function name, a list of input
columns, and physical type information. This chapter adds the boundary between those two views:

```text
runtime call                         typed work
"i32_add" + [left, right]  ->  BinaryExpression<I32Add>  ->  evaluate_binary
```

The erased layer owns function metadata and arity checking. The typed layer keeps physical-type
checks, length checks, null propagation, output construction, and the row loop.

## Starting Point and Result

Continue from your Chapter 3 implementation and copy the next supplied contract:

```console
cargo x copy-test --chapter 4
cargo test -p type-exercise-starter chapter_4 --locked
```

After this chapter, add these public pieces:

- `Expression`, an object-safe runtime interface;
- `BinaryExpression<F>`, which adapts a typed binary function to that interface;
- `StringConcat`, a borrowed-string function with owned `String` output;
- `BUILTIN_EXPRESSION_NAMES`, the generated list of concrete physical kernels; and
- `build_builtin_expression`, which constructs a boxed expression by runtime name.

Extend `ExpressionError` with `InputArityMismatch { expected, actual }`. Keep the Chapter 3 API
unchanged: the adapter must delegate to `evaluate_binary` rather than copy its row loop.

## Make the Runtime Interface Object-Safe

The executor needs to store different concrete adapters behind one trait object. Use methods whose
signatures do not mention `Self` or introduce generic type parameters:

```rust,ignore
pub trait Expression {
    fn name(&self) -> &'static str;
    fn input_types(&self) -> &[PhysicalType];
    fn arity(&self) -> usize {
        self.input_types().len()
    }
    fn output_type(&self) -> PhysicalType;
    fn evaluate(
        &self,
        inputs: &[ColumnViewImpl<'_>],
    ) -> Result<ArrayImpl, ExpressionError>;
}
```

The metadata is part of the runtime contract:

| Name | Input types | Output type |
| --- | --- | --- |
| `i32_add` | `[Int32, Int32]` | `Int32` |
| `string_concat` | `[String, String]` | `String` |

These names describe concrete physical kernels. Do not add a logical overloaded name such as
`add`, implicit casts, or signature selection yet. Chapter 5 will bind logical calls to one of
these physical expressions.

## Adapt Without Duplicating the Typed Loop

`BinaryExpression<F>` stores three things:

```rust,ignore
pub struct BinaryExpression<F> {
    name: &'static str,
    input_types: [PhysicalType; 2],
    function: F,
}
```

Its constructor derives both input types from `F::Left` and `F::Right`. The output type comes from
`F::Output`. The `Expression` implementation follows this order:

1. Compare `inputs.len()` with `2`.
2. Return `InputArityMismatch` before indexing if they differ.
3. Pass `inputs[0]` and `inputs[1]` to `evaluate_binary`.

The order matters. Indexing first would panic on a missing input, while converting a one-input call
first could report a misleading physical-type error instead of the arity error at the erased
boundary.

The adapter's `where` clause repeats the checked erased-to-typed conversions required by
`evaluate_binary` for every input lifetime. Do not convert through `ScalarImpl`, allocate temporary
columns, or add a second row loop.

## Exercise Borrowed Inputs and Owned Output

Integer addition alone cannot prove that the erased adapter preserves generic scalar families. Add
a second scalar function:

```rust,ignore
pub struct StringConcat;

impl BinaryScalarFunction for StringConcat {
    type Left = String;
    type Right = String;
    type Output = String;

    fn evaluate(&self, left: &str, right: &str) -> String {
        // Build one owned result from two borrowed inputs.
        todo!()
    }
}
```

Both inputs borrow `&str` values from their views. Each non-null row creates one owned `String` for
the output array. Null rows must still skip the scalar function. A dictionary string input and a
constant suffix therefore evaluate without materializing either input representation:

```text
dictionary rows: ["data", "rust", null, null]
constant suffix: ["base", "base", "base", "base"]
result:          ["database", "rustbase", null, null]
```

## Generate the Builtin Catalog

The two builtins repeat the same catalog entry: a name, a scalar function value, and a boxed
`BinaryExpression`. Use one small declarative macro to generate both the public name list and the
runtime constructor:

```rust,ignore
define_builtin_expressions! {
    "i32_add" => I32Add,
    "string_concat" => StringConcat,
}
```

The expansion should provide this behavior:

```text
build_builtin_expression("i32_add")        -> Some(Box<dyn Expression>)
build_builtin_expression("string_concat")  -> Some(Box<dyn Expression>)
build_builtin_expression("add")            -> None
build_builtin_expression("missing")        -> None
```

Returning `None` for an unknown physical name is deliberate. Chapter 5 will add a richer logical
binding error once there are signatures and overload selection to report.

The macro removes repetitive catalog arms; it does not generate a different evaluation loop for
each function. Every binary builtin still reaches the single Chapter 3 loop through
`BinaryExpression<F>`.

## Keep Error Ownership Layered

Each boundary reports the failure it can identify:

| Failure | Owning boundary | Error/result |
| --- | --- | --- |
| unknown physical name | builtin catalog | `None` |
| wrong number of inputs | erased adapter | `InputArityMismatch` |
| wrong input physical type | typed conversion | `TypeMismatch` |
| unequal logical lengths | typed evaluator | `InputLengthMismatch` |
| null row | typed row loop | null output, not an error |

This layering keeps the runtime adapter small and prevents error precedence from depending on
which input happened to be inspected first.

## Implementation Checkpoints

1. Add `InputArityMismatch` and its `Display` message to `ExpressionError`.
2. Define the object-safe `Expression` metadata and evaluation methods.
3. Implement `BinaryExpression<F>` with an arity check before slice indexing.
4. Add `StringConcat` and preserve strict null propagation through the Chapter 3 loop.
5. Generate the two-name builtin catalog with one macro invocation.
6. Run the focused contract and keep Chapters 1–3 green.

## Review Your Chapter Result

Run:

```console
cargo test -p type-exercise-starter chapter_4 --locked
cargo test -p type-exercise-starter --lib --locked
```

The Chapter 4 contract contains seven tests. They cover trait-object evaluation and metadata,
generated catalog lookup, dictionary/constant string evaluation, strict null short-circuiting,
unknown names, arity precedence, physical-type errors, and length errors.

Before continuing, explain:

- why `Expression` can be used as `dyn Expression`;
- which work belongs to `BinaryExpression<F>` and which remains in `evaluate_binary`;
- why `StringConcat` takes borrowed inputs but returns an owned output;
- why arity must be checked before indexing or converting the input slice; and
- why the catalog exposes physical names instead of performing logical overload resolution.

Chapter 5 will add that missing logical layer: it will validate signatures and bind a logical call
to one concrete expression from this catalog.

{{#include copyright.md}}
