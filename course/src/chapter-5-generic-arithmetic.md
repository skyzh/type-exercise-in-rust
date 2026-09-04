{{#include wip-banner.md}}

# Chapter 5: Specialize Common Column Shapes

The shared Chapter 3 fallback calls `ColumnView::get(row)` so Array, Constant, and Indexed inputs
all behave correctly. That generality also repeats representation dispatch inside every row. This
chapter moves only the common shapes into concrete loops and keeps the typed fallback as the
semantic authority.

```console
cargo x copy-test --chapter 5
cargo test -p type-exercise-starter-supplied-tests chapter_5 --locked
```

## Give a loop one concrete input shape

In `core/src/column.rs`, let the expression module see the private typed representation enum. Do
not expose it outside the core crate. Then add private Array and Constant accessors whose concrete
types are known to the compiler before the loop begins.

Build three public adapters in `core/src/expression.rs`:

- `auto_vectorize_unary` specializes Array and Constant, with Indexed on `ColumnView::get`;
- `auto_vectorize_binary` specializes Array/Array, Array/Constant, Constant/Array, and
  Constant/Constant, with any Indexed input on the fallback; and
- `auto_vectorize_ternary` specializes the common Array/Array/Array shape and sends every other
  combination to the fallback.

Validate arity, physical families, and lengths before selecting a shape. A concrete loop still
propagates strict nulls and builds the output array associated with its generic output scalar. It
must not select a numeric operator or logical function name inside the row loop.

This is deliberately selective. Generating all 27 ternary Array/Constant/Indexed combinations
would increase code size without removing Indexed's indirect lookup. One common ternary path and
the complete binary Array/Constant cross-product capture the useful boundary.

Run both commands again:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_5 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
```

The focused tests cover unary, binary, and ternary dense shapes plus Indexed fallback. The results
must match Chapter 3's nullable behavior. [Chapter 6](./chapter-6-systematic-arity.md) adds one more
bounded physical lane and keeps operations with exceptional semantics on the general path.

{{#include copyright.md}}
