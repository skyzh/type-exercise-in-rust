# Expand Numeric Families, Customize Everything Else

The framework can vectorize any scalar function with a supported signature. That does not mean the
framework should generate every function for every physical type.

## Numeric Addition

Addition has a real combination problem. The binder must select both a kernel and an output type:

```text
SmallInt + SmallInt -> SmallInt
SmallInt + Integer  -> Integer
Integer  + BigInt   -> BigInt
Integer  + Real     -> Double
Real     + Double   -> Double
```

The scalar implementation is generic:

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

`for_all_arithmetic_combinations!` is the database policy. A second macro turns each row into a
`BinaryExpression` instantiation. The compiler generates the concrete loops.

## Comparisons

Comparisons have a similar input cross-product but always return Boolean. Each table entry specifies
the common comparison type:

```text
{ int16,   int64,   int64 }
{ int32,   float32, float64 }
{ fwchar,  varchar, varchar }
```

The four comparison functions (`<=`, `>=`, `=`, `!=`) reuse the same table. The binder rejects
combinations absent from it instead of reaching `unimplemented!()` during execution.

## Logical and Physical Associations

Macros such as `int32!`, `varchar!`, and `fwchar!` associate three facts:

```text
logical match pattern, concrete array type, concrete scalar type
```

For example, both `varchar!` and `fwchar!` name `StringArray` and `String`, but provide different
`DataType` patterns. This lets comparison policy include `CHAR`/`VARCHAR` combinations without
duplicating physical kernels.

Rust macros expand from the outside inward. The type macros therefore use a callback shape:

```rust
macro_rules! varchar {
    ($callback:ident) => {
        $callback!(DataType::Varchar, StringArray, String)
    };
}
```

An extractor callback selects the match pattern, array, or scalar token. Keeping this explanation
next to its database use is more useful than treating callback macros as an isolated trick.

## Custom Data-Type-Specific Expressions

`str_contains` is intentionally not in a giant function/type matrix:

```rust
fn str_contains(left: &str, right: &str) -> bool {
    left.contains(right)
}
```

Its factory accepts string logical types and constructs one
`BinaryExpression<String, String, bool, _>`. A JSON accessor might accept a JSON value and a string
path; a date truncation function might store a unit parameter; a list function might return a
borrowed element through a specialized writer. Each should register the signatures it really
supports.

In real database systems, this explicit category is the majority. Generic type exercise is
infrastructure for the dense families, not a tax on every new expression.

## Design Checklist for a New Function

Before adding a macro entry, ask:

1. Does the function have many meaningful input-type combinations?
2. Do those combinations share one implementation and null policy?
3. Is promotion policy stable and reviewable as a table?
4. Would an explicit typed kernel be shorter and clearer?

If the answer to the last question is yes, use a custom factory.

## Task

Trace one numeric signature such as `SmallInt + Integer` through the arithmetic combination table,
the callback type macros, the binder match, and the resulting concrete `PrimitiveBinaryExpression`.
Then trace `str_contains` and identify exactly where it leaves the generated family. Explain why
adding the string function to the numeric matrix would weaken the policy boundary.

Run `cargo test -p expr-impl` to check both a supported promotion and an unsupported signature. Do
not add a new physical type merely to exercise a macro; that would require array, scalar, binder,
and diagnostic work outside this task.

Continue to [binding and executing the complete framework](./framework.md).

{{#include ../copyright.md}}
