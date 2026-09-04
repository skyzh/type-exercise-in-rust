{{#include wip-banner.md}}

# Chapter 10: Share and Schedule a Batch Safely

The completed engine evaluates one deterministic batch synchronously. A worker pool needs to share
the registry and erased expressions; an async executor needs one future around the same batch.
Neither boundary should clone column contents or move dynamic dispatch into scalar rows.

```console
cargo x copy-test --chapter 10
cargo test -p type-exercise-starter-supplied-tests chapter_10 --locked
```

## Make shared factories reusable

Strengthen `Expression` to `Any + Send + Sync`. In `expr/src/binder.rs`, require the stored factory
trait object and every registered factory to be `Fn + Send + Sync + 'static`. Keep `Fn`: binding
is reusable and shared access must not require mutable closure state. Thread-safe captured state
can use `Arc`, atomics, or locks outside the registry.

The existing `Array::iter` remains opaque and tied to the array borrow. `ColumnViewImpl<'a>` remains
covariant so a long-lived view can be reborrowed for one shorter evaluation. Do not expose a
concrete iterator, add `unsafe`, or force borrowed input into `'static` storage.

## Box only the complete batch future

Add the static and erased boundaries:

```rust,ignore
pub fn evaluate_static<'a, E>(
    expression: &'a E,
    inputs: &'a [ColumnViewImpl<'a>],
) -> impl Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a;

pub type BatchFuture<'a> =
    Pin<Box<dyn Future<Output = anyhow::Result<ArrayImpl>> + Send + 'a>>;

pub trait AsyncExpression: Send + Sync {
    fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a>;
}
```

`AsyncExpressionAdapter` owns one `Box<dyn Expression>` and calls its synchronous `evaluate`
exactly once when polled. `BoundExpression` forwards through the same boundary without rebinding.
The future lifetime covers the expression, input slice, and arrays borrowed by every view.

Run the completed course:

```console
cargo test -p type-exercise-starter-supplied-tests --locked
cargo test -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The final tests compare synchronous, static-future, erased-future, and bound-future results; require
thread-safe registries and expressions; and preserve validation and scalar failures unchanged.

You now have one path from physical storage to a scheduled batch: typed arrays, lazy views, shared
fallbacks, selective specialization, one erased expression shell, one binder, one-level Lists, and
one async boundary around the complete operation.

{{#include copyright.md}}
