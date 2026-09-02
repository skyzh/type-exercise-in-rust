{{#include wip-banner.md}}

# Chapter 14: Add a Batch Async Boundary

Evaluation is synchronous, but an execution engine may need to schedule a complete batch through
an asynchronous interface. This chapter wraps the existing operation exactly once. It does not
make scalar functions async, yield between rows, or change validation and error semantics.

## The learner-owned boundary

```console
cargo x copy-test --chapter 14
cargo test -p type-exercise-starter-supplied-tests chapter_14 --locked
```

Add three layers in the core expression framework.

### A statically typed future

`evaluate_static` borrows one `Expression` and its input views, then returns
`impl Future<Output = anyhow::Result<ArrayImpl>> + Send`. The future calls synchronous
`evaluate` once when polled. Its lifetime covers the expression, the view slice, and every array
borrow reachable through those views.

### An erased future boundary

Define:

```rust,ignore
pub type BatchFuture<'a> =
    Pin<Box<dyn Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a>>;

pub trait AsyncExpression: Send + Sync {
    fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
}
```

`AsyncExpressionAdapter` owns an existing `Box<dyn Expression>` and boxes only the complete batch
future. The selected scalar kernel and row loop stay synchronous and typed inside it.

### A bound-expression convenience

Forward `BoundExpression::evaluate_async` through the same physical expression. Binding remains a
one-time logical step; the async method does not resolve a name or choose a kernel again.

Run the completed course:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_14 --locked
cargo test -p type-exercise-starter-expr --doc --locked
cargo test -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The nine focused tests prove equal synchronous/static/erased/bound results, exactly one underlying
evaluation for every async path, propagation of scalar and validation failures, `Send`
futures, and a borrow lifetime that covers expression, views, and arrays.

## Why the future is the outer boundary

The expression engine already performs one deterministic batch operation. Boxing a future around
that operation gives a scheduler one uniform interface without imposing async dispatch on each
row. Static callers keep an opaque compiler-known future; erased callers pay boxing once per
batch. Both paths preserve the synchronous implementation as the single semantic authority.

You have now connected the full path: physical families, borrowed views, scalar operations,
reusable vectorizers, dense selection, SQL null semantics, runtime erasure, transactional strings,
logical binding, nested storage, Rust ownership, and an async batch boundary.

{{#include copyright.md}}
