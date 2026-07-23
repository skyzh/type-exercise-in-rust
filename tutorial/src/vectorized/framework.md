# Bind and Execute the Complete Framework

The pieces now form one end-to-end path. This example binds a custom string function and evaluates
it without materializing either a dictionary column or a constant column:

```rust
let registry = FunctionRegistry::with_builtins();
let expression = registry.bind_binary(
    "contains",
    DataType::Varchar,
    DataType::Varchar,
)?;

let dictionary: ArrayImpl = StringArray::from_slice(&[
    Some("rust"),
    Some("database"),
    Some("type system"),
]).into();
let indices = [Some(0), Some(1), None, Some(2)];

let left = ColumnViewImpl::dictionary(&indices, &dictionary)?;
let right = ColumnViewImpl::constant(
    ScalarRefImpl::String("a"),
    indices.len(),
);

let result = expression.eval(&[left, right])?;
```

The result is `[false, true, null, false]`.

## What Happens Once per Expression

`FunctionRegistry::bind_binary`:

1. finds the planning-time factory for `contains`;
2. checks that both logical types use the string physical representation;
3. constructs `BinaryExpression<String, String, bool, str_contains>`; and
4. records Boolean as the logical result type.

Numeric factories additionally select a promotion-table entry and construct a
`PrimitiveBinaryExpression`. String functions continue to use the general `BinaryExpression`.

## What Happens Once per Batch

`BoundExpression::eval` and the generated expression:

1. verify the input count and physical types;
2. convert `ColumnViewImpl` to `ColumnView<String>`;
3. dispatch the left dictionary and right constant representations; and
4. allocate one Boolean output builder.

## What Happens Once per Row

The monomorphized loop:

1. maps the dictionary key to `&str`;
2. reads the repeated constant `&str`;
3. applies strict null propagation;
4. calls `str_contains`; and
5. pushes the Boolean result.

No logical type, function name, type-erased scalar, or view enum is matched per row.

For a primitive numeric or comparison expression, the batch step also checks the cached null count.
If every regular-array input is valid, the expression reads contiguous value slices and initializes
output validity in bulk. If any input is nullable—or if the input is a dictionary—the same object
delegates to the nullable loop described above. Planning does not duplicate function signatures for
these paths.

## Extending the Registry

A custom binary factory has this shape:

```rust
registry.register_binary("my_function", |left, right| {
    // Check DataType values and choose a typed Expression implementation.
    // Return BoundExpression::new(expression, [left, right], output_type).
});
```

The public `BoundExpression::new` constructor is the escape hatch that keeps the registry open to
custom expression objects. A production system would likely add richer function properties,
overload priorities, cast insertion, variadic factories, and serialization-friendly function IDs.
Those features can grow around the same planning/runtime boundary.

## Compatibility

Existing code can continue to call:

```rust
let expression = build_binary_expression(function, left_type, right_type);
let result = expression.eval_expr(&[&left_array, &right_array])?;
```

This helper panics on an unsupported signature to preserve the original API. New code should call
the fallible binder or registry and handle `BindError` during planning.

## Framework Checkpoint

You have built a framework with:

- logical type checking before runtime;
- static scalar and accessor types inside hot loops;
- runtime type erasure at stable boundaries;
- reusable strict vectorization;
- efficient constant and dictionary inputs;
- generated numeric/comparison families; and
- a batch-selected, SIMD-friendly primitive fast path;
- explicit extension points for data-type-specific expressions.

The final chapter checks whether the abstractions remain [competitive with hand-written loops](../benchmarks.md).

## Test Your Understanding

- Which errors should be returned by the binder, and which can still occur during evaluation?
- How would you add a stateful regular-expression kernel without changing `ColumnView`?
- Why does preserving the array-only adapter help an incremental migration?

{{#include ../copyright.md}}
