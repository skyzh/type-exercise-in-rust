# Day 5: Add a Batch Async Boundary

Imagine an external function that sends one columnar batch to a service and waits for one response.
The wait belongs around the batch. Turning every row into a separate future would multiply
scheduling, allocation, cancellation, and error-handling work inside the hot loop built during the
first four days.

Today you will wrap a synchronous expression in one future per batch and compare the static and
type-erased future interfaces that Rust offers.

## Starting Point and Result

Day 4 made `Expression` a synchronous, reusable, thread-safe interface. `eval` borrows a slice of
column views and returns one materialized `ArrayImpl`.

After this day:

- `eval_static` returns one compiler-known future with return-position `impl Future`;
- `AsyncExpression` is dyn-compatible and returns a pinned boxed future;
- the future borrows both the expression and its input views for one shared lifetime;
- an adapter can expose any synchronous `Expression` through the async interface; and
- focused tests drive the no-I/O future to completion without adding an async runtime.

This example deliberately performs no external I/O. It teaches the boundary and its lifetime,
pinning, and type-erasure requirements. Timeouts, cancellation policy, retries, concurrency limits,
and remote serialization are production concerns for a later executor.

## One Future for the Observable Operation

The synchronous operation is already batch-shaped:

```rust,ignore
fn eval(
    &self,
    inputs: &[ColumnViewImpl<'_>],
) -> anyhow::Result<ArrayImpl>;
```

The async adapter should preserve that shape:

```text
borrow expression + all input views
                 |
                 v
          create one future
                 |
                 v
      await one batch operation
                 |
                 v
        one output ArrayImpl
```

Null propagation, dictionary access, primitive fast-path selection, and scalar kernel calls remain
inside the synchronous expression. The adapter does not create a future for any row.

## Static Future Type

When the caller knows the concrete expression type, the function can hide its concrete async state
machine:

```rust,ignore
fn eval_static<'a, E: Expression>(
    expression: &'a E,
    data: &'a [ColumnViewImpl<'a>],
) -> impl Future<Output = Result<ArrayImpl>> + Send + 'a {
    async move { expression.eval(data) }
}
```

This is return-position `impl Trait` (RPIT). The compiler chooses one concrete future type for the
function. The caller can await it, but does not name its generated state-machine type.

The shared `'a` is part of the contract: the future may borrow the expression, the slice, the views,
and the arrays behind those views. None may be dropped before the future completes.

## Erase the Future Behind a Trait Object

A registry may store different asynchronous expression implementations behind
`Box<dyn AsyncExpression>`. A trait-object method cannot return a different unnamed concrete future
for each implementation through this interface, so erase the future too:

```rust,ignore
type BatchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ArrayImpl>> + Send + 'a>>;

trait AsyncExpression: Send + Sync {
    fn eval_async<'a>(
        &'a self,
        data: &'a [ColumnViewImpl<'a>],
    ) -> BatchFuture<'a>;
}
```

`Box` supplies a stable allocation and erases the concrete future type. `Pin` states that the
future will not move after polling begins, which supports generated futures that may contain
self-references. `Box::pin` creates both together.

Do not read `Pin` as “this future is definitely self-referential,” and do not assume every future
is `!Unpin`. `Unpin` means a type does not rely on pinning for movement safety. The erased
interface accepts either kind by promising not to move the future through its pinned handle.

## Polling Without a Runtime

The example's adapter immediately calls synchronous code inside `async move`, so its future should
return `Poll::Ready` on the first poll. A test can use a no-op `Waker`:

```rust,ignore
fn poll_ready<F: Future + ?Sized>(mut future: Pin<&mut F>) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the example future should complete without I/O"),
    }
}
```

This helper is valid only because the exercise promises no I/O and no pending work. A real external
function needs an executor that will wake and poll the future again.

## The Async-Boundary Contract

Implement and preserve these rules in `expr-common/examples/async_expression.rs`:

1. One call creates one future for the complete batch.
2. The future output is the same `Result<ArrayImpl>` produced by the synchronous expression.
3. The future is `Send`; the async expression object is `Send + Sync`.
4. The future cannot outlive the borrowed expression, view slice, or arrays behind the views.
5. Static and erased future paths produce the same values for the same valid input.
6. Wrong arity and unsupported view representations remain errors, not panics or placeholder
   outputs.
7. The example uses no runtime, network, timer, background thread, or unsafe pin projection.
8. The core `Expression` trait and per-row evaluator remain synchronous.

Using one batch future and a pinned boxed erased return type is the selected course design. The
identity expression is only a small observable example; a real adapter may perform I/O while
preserving the same boundary.

## Implementation Checkpoints

Work in this order:

1. Implement a one-input synchronous identity expression with explicit arity and representation
   errors.
2. Add `eval_static` and confirm its future borrows the inputs.
3. Define `BatchFuture` and dyn-compatible `AsyncExpression`.
4. Implement `AsyncAdapter<E: Expression>` with `Box::pin`.
5. Poll both future forms in focused tests and compare their output.
6. Test the non-array-view and wrong-arity errors.

Keep this work in the example. Do not add an async runtime dependency or change the production
expression evaluator.

## Verify the Day

Run:

```console
cargo test -p expr-common --example async_expression --locked
cargo test --workspace --all-targets --locked
```

The example tests should pass one regular array through both future forms and reject a constant
view and an empty input list at the async boundary.

## Review the Complete Course Result

Trace one expression from planning through execution:

1. Day 2 binds a function name and logical types to a typed kernel.
2. Day 1 converts physical arrays, constants, or dictionaries to typed borrowed views.
3. Day 3 selects a dense primitive loop only when the batch proves it is eligible.
4. Day 4 guarantees that erased expressions can cross executor threads and that borrows remain
   valid.
5. Day 5 optionally wraps the whole synchronous evaluation in one future.

Explain where each error is detected, which work occurs once per expression, batch, and row, and
which production async concerns remain outside this repository. If you can do that from the code
and focused tests, you have the framework's central mental model—not only a passing workspace.

{{#include copyright.md}}
