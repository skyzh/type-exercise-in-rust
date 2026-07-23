# Generate Vectorized Function Templates

The scalar function should remain the smallest unit of expression logic:

```rust
fn str_contains(left: &str, right: &str) -> bool {
    left.contains(right)
}
```

`BinaryExpression<I1, I2, O, F>` turns that function into an object-safe batch expression. Its
important bound is direct:

```rust
F: Fn(I1::RefType<'_>, I2::RefType<'_>) -> O
```

Modern stable Rust accepts this higher-ranked behavior for ordinary and generic functions.

## Typed Evaluation

After erased views are converted and their encodings dispatched, the generated hot loop is
equivalent to:

```rust
fn eval_typed<'a, V1, V2>(&self, left: V1, right: V2) -> Result<ArrayImpl>
where
    V1: ColumnAccessor<'a, I1>,
    V2: ColumnAccessor<'a, I2>,
{
    if left.len() != right.len() {
        bail!("column length mismatch");
    }

    let mut output = O::ArrayType::Builder::with_capacity(left.len());
    for row in 0..left.len() {
        match (left.get(row), right.get(row)) {
            (Some(left), Some(right)) => {
                output.push(Some((self.func)(left, right).as_scalar_ref()));
            }
            _ => output.push(None),
        }
    }
    Ok(output.finish().into())
}
```

This function contains no `DataType`, `PhysicalType`, `ArrayImpl` match, or function-name lookup.

## Why Generate Arity Templates?

Rust has no variadic generics. Unary and binary expression structs are easy to write, but repeating
the same null and dispatch code through arity five is error prone. `expr-template-impl` builds Rust
syntax for `FnArgs1Expression` through `FnArgs5Expression`, and `build.rs` writes the generated
modules.

The generator also emits all array/constant/dictionary representation combinations. For two inputs
that is `3^2 = 9` one-time dispatch arms; for five inputs it is `3^5 = 243`. The source is larger,
but the inner loop in each arm uses static dispatch.

This is an intentional code-size/performance tradeoff. A production system might cap generated
arity, box uncommon accessors, or use a specialized binary fast path plus a generic fallback.

## Compatibility Adapter

The revised expression exposes two entry points:

```rust
expr.eval_views(ColumnViewImpl::Array(...), ColumnViewImpl::Constant(...));
expr.eval_batch(&left_array_impl, &right_array_impl);
```

`eval_batch` retains the original course API and wraps arrays as views. Existing code can migrate
incrementally.

## Writer Kernels and Custom Loops

Some functions do not naturally return one owned scalar. A string concatenation kernel may write
directly into a string builder to reuse capacity; a regular-expression function may store compiled
state; a boolean expression may short-circuit. Those functions can implement `Expression` directly
or use another template.

The generated strict template is a convenience for a common shape, not a requirement imposed on
every expression.

## Task

Inspect `expr-template-impl/src/lib.rs` and the generated binary file. Find:

1. the generic `Fn` bound;
2. the physical downcast bounds;
3. the one-time encoding match;
4. the strict null branch; and
5. the `Expression` trait implementation.

Next, decide [which functions deserve generic expansion](./data_types.md).

{{#include ../copyright.md}}
