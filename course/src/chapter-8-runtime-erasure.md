{{#include wip-banner.md}}

# Chapter 8: Bind Logical Calls Once

SQL starts with a logical name and logical argument types. Evaluation starts with one physical
expression. Binding is the one-time step that connects them; a row loop must never repeat name
lookup, numeric promotion, or kernel selection.

```console
cargo x copy-test --chapter 8
cargo test -p type-exercise-starter-supplied-tests chapter_8 --locked
```

## Build the physical catalog, then bind once

Enable `core/src/promotion.rs` and implement the explicit lossless table for SmallInt, Integer,
BigInt, Real, and Double. Reject BigInt/float pairs and every Decimal arithmetic pair whose scale
and rounding semantics are unspecified.

Now expand `expr/src/numeric.rs` from its Chapter 3 examples and enable `boolean.rs` and
`string.rs`. Each module owns concrete scalar semantics and exposes crate-private factories that
choose a complete batch kernel, its fixed arity, and physical input/output metadata before building
a `BatchExpression<N>`. This is the first checkpoint with that physical catalog; Chapter 7 kept
only the shell and a test-local kernel.

In `expr/src/binder.rs`, implement:

- `BindError` for unknown names, wrong arity, unsupported logical signatures, and inconsistent
  physical metadata;
- `BoundExpression`, which stores logical input/output types beside one `Box<dyn Expression>`;
  and
- `FunctionRegistry`, whose slice-based factory map is the sole name-to-expression registry.

`register_unary`, `register_binary`, and `register_ternary` adapt their closures to the same slice
factory after checking arity. `with_builtins` registers arithmetic, negation, clamp, comparisons,
String containment/concatenation, and three-valued Boolean operations. Logical Char and Varchar
may share String storage without becoming the same logical type.

Binding follows one order: resolve the name, check arity, apply logical coercion, build one physical
expression, validate its metadata, and store it. `BoundExpression::evaluate` then delegates the
complete batch directly.

```console
cargo test -p type-exercise-starter-supplied-tests chapter_8 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The tests bind and evaluate the introduced numeric, Boolean, and borrowed String factories through
the public registry, then reject unknown names, wrong arities, and lossy signatures before
evaluation. [Chapter 9](./chapter-9-binding-coercion.md) adds one nested storage shape without
changing the expression boundary.

{{#include copyright.md}}
