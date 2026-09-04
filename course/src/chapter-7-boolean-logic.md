{{#include wip-banner.md}}

# Chapter 7: Erase One Complete Batch

A planner needs to keep expressions with different scalar types and arities in one collection.
Erasing a scalar operation inside every row would discard the specialization built so far. Erase
the complete batch instead.

```console
cargo x copy-test --chapter 7
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
```

## Store one preselected kernel

Define the shared function-pointer shape:

```rust,ignore
pub type BatchKernel =
    for<'a> fn(&[ColumnViewImpl<'a>]) -> anyhow::Result<ArrayImpl>;
```

Then add `BatchExpression<const N: usize>`. It stores a static name, `[PhysicalType; N]`, one
output family, and one `BatchKernel`. Its inherent `evaluate` method validates input count,
families, and lengths before calling the kernel, then rejects an output whose physical family or
row count disagrees with the declared contract.

Add the object-safe `Expression` trait with discoverable name, input types, arity, output type, and
one complete-batch `evaluate` method. Implement it for every `BatchExpression<N>`. Keep
`BinaryExpression`, `PrimitiveBinaryExpression`, `PrimitiveLoop`, and `evaluate_with_loop`
absent—there is one shell and one traversal layer.

The facade now chooses concrete numeric, Boolean, and String kernels before constructing the
shell. The row loop receives no operator enum and performs no logical lookup.

```console
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The focused tests use unary and binary shells through `dyn Expression`, and exercise arity, type,
length, output-family, and output-length rejection. [Chapter 8](./chapter-8-runtime-erasure.md)
binds logical calls to these already-selected physical expressions.

{{#include copyright.md}}
