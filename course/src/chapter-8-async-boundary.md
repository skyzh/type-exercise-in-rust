# Add a Batch Async Boundary

Chapter 7 made expressions safe to share with executor workers. This chapter adds an optional
asynchronous interface around one complete batch while keeping the expression engine itself
synchronous.

That placement matters. A remote function may eventually wait on one request for one batch. It
does not need one future per row. The existing evaluator already owns the correct row loop, null
propagation, dictionary access, primitive-loop selection, and typed errors, so the async boundary
should delegate to it exactly once.

## Starting Point and Public Pieces

Continue from your completed Chapter 7 implementation. Copy the Chapter 8 contract and run it once:

```console
cargo x copy-test --chapter 8
cargo test -p type-exercise-starter chapter_8 --locked
```

The first run should fail to compile because the async types and methods do not exist yet. A
successful run with zero matched tests means the contract was not copied.

Add these public pieces without changing synchronous evaluation:

- `evaluate_static`, which returns one compiler-known future with return-position `impl Future`;
- `BatchFuture<'a>`, the pinned boxed future used at the erased boundary;
- a dyn-compatible `AsyncExpression` trait;
- `AsyncExpressionAdapter`, which owns an existing `Box<dyn Expression>`; and
- an `AsyncExpression` implementation for `BoundExpression`.

Both future paths return the existing `Result<ArrayImpl, ExpressionError>`. Do not introduce a new
error wrapper or translate failures into strings.

## Put One Future Around One Batch

The synchronous operation is already batch-shaped:

```text
borrow expression + input views + arrays
                    |
                    v
             create one future
                    |
                    v
       call synchronous evaluate once
                    |
                    v
       Result<ArrayImpl, ExpressionError>
```

The future does not make scalar work asynchronous. It adds a boundary where a production adapter
could later perform one batch request. This course version deliberately adds no runtime, I/O,
timer, retry, background thread, or per-row future.

## Keep the Static Future Type

When the caller does not need to erase the future, return-position `impl Trait` keeps its generated
state-machine type known to the compiler:

```rust,ignore
fn evaluate_static<'a, E>(
    expression: &'a E,
    inputs: &'a [ColumnViewImpl<'a>],
) -> impl Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a
where
    E: Expression + ?Sized,
{
    async move { expression.evaluate(inputs) }
}
```

The shared `'a` covers the expression, the slice of views, and the arrays or scalar values borrowed
by those views. The returned future cannot outlive any of them. `?Sized` also permits a borrowed
`dyn Expression`; “static” here describes the future representation, not whether the physical
expression was type-erased earlier.

The explicit `Send` bound makes the executor-facing promise visible. It does not spawn a worker or
require the future to be `'static`.

## Erase the Future for Dynamic Dispatch

Different `AsyncExpression` implementations may create different future types. A trait-object
method therefore returns one erased future type:

```rust,ignore
type BatchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ArrayImpl, ExpressionError>> + Send + 'a>>;

trait AsyncExpression: Send + Sync {
    fn evaluate_async<'a>(
        &'a self,
        inputs: &'a [ColumnViewImpl<'a>],
    ) -> BatchFuture<'a>;
}
```

`Box` gives the unknown future a stable allocation and erases its concrete type. `Pin` prevents the
future from moving through that handle after polling starts. The interface supports generated
futures whether or not a particular future actually contains self-references.

The method is dyn-compatible: its generic parameter is only a lifetime, not a caller-selected type.
The resulting object remains `Send + Sync`, matching the Chapter 7 worker boundary.

## Reuse the Existing Physical Expression

`AsyncExpressionAdapter` should own the `Box<dyn Expression>` already produced by the physical
catalog. Its implementation boxes one future and delegates once:

```rust,ignore
fn evaluate_async<'a>(&'a self, inputs: &'a [ColumnViewImpl<'a>]) -> BatchFuture<'a> {
    Box::pin(async move { self.expression.evaluate(inputs) })
}
```

Do not repeat arity checks, type conversions, null logic, or loop selection in the adapter. The
synchronous expression remains the single source of those semantics. The boxed allocation occurs
once per erased batch future, not once per row.

Apply the same forwarding rule to `BoundExpression`. It already represents the planning-to-
execution handoff, so implementing `AsyncExpression` for it lets a caller bind a logical function
and then choose either synchronous or asynchronous batch evaluation without rebinding or changing
the physical expression.

## Poll the No-I/O Future Once

The course adapter performs synchronous work inside `async move` and contains no await point. Its
future must therefore return `Poll::Ready` on its first poll. A focused test can prove that without
adding an executor dependency:

```rust,ignore
fn poll_ready<F: Future + ?Sized>(mut future: Pin<&mut F>) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the no-I/O batch future must complete on its first poll"),
    }
}
```

This helper is specific to the no-I/O exercise. A real future may return `Pending`, arrange a wake,
and require an executor to poll it again.

Use a counting expression to distinguish one batch delegation from duplicated work. Creating the
future should not call `evaluate`; polling the ready future should call it exactly once.

## Preserve Errors and Borrowing

The adapter must return the exact error from synchronous evaluation. Test the existing precedence
with three separate inputs:

1. the wrong number of views returns `InputArityMismatch`;
2. a wrong physical type returns `TypeMismatch`; and
3. two correctly typed views with different lengths return `InputLengthMismatch`.

Do not replace these variants with an async-specific error. Nothing failed in future scheduling;
the existing expression contract rejected the batch.

Also spell helper signatures with the shared lifetime and compile them for borrowed array,
constant, and dictionary views. The future must not become `'static` by cloning or leaking borrowed
data. Its output is owned, but its evaluation inputs remain borrowed until completion.

## The Chapter Contract

Preserve these rules:

1. One call creates one future for the complete batch.
2. Static, erased, bound, and synchronous paths produce the same output.
3. The future is `Send`; an erased async expression is `Send + Sync`.
4. One ready future invokes synchronous evaluation exactly once.
5. The future lifetime covers the expression, view slice, and arrays behind the views.
6. Arity, type, and length failures remain the exact existing `ExpressionError` variants.
7. Row kernels, null behavior, dictionary behavior, binding, and primitive-loop selection remain
   synchronous and unchanged.
8. No runtime, I/O, timer, retry, background thread, unsafe pin projection, or per-row future is
   added.

## Implementation Checkpoints

1. Import `Future` and `Pin`, then define the `BatchFuture<'a>` alias.
2. Add `evaluate_static` with one lifetime shared across the expression, views, and borrowed data.
3. Define the dyn-compatible `AsyncExpression` trait.
4. Adapt one existing `Box<dyn Expression>` with `AsyncExpressionAdapter` and `Box::pin`.
5. Implement `AsyncExpression` for `BoundExpression` by forwarding to its synchronous method.
6. Add the one-poll helper and compare static, erased, bound, and synchronous results.
7. Assert the worker traits, exact errors, shared borrow lifetime, and one synchronous call.
8. Keep every earlier chapter green.

## Review Your Chapter Result

Run:

```console
cargo test -p type-exercise-starter chapter_8 --locked
cargo test -p type-exercise-starter --lib --locked
cargo test -p type-exercise-starter --doc --locked
```

The Chapter 8 contract contains six supplied tests. They cover static result preservation, erased-
future equivalence and worker traits, bound-expression forwarding, one ready poll and one
synchronous call, exact arity/type/length errors, and the shared borrowed lifetime.

Before finishing, explain:

- why the future belongs around the batch rather than each row;
- what differs between the RPIT future and `BatchFuture`;
- why `Pin<Box<dyn Future + Send + 'a>>` does not imply `'static`;
- why the no-op-waker helper is valid only for this no-I/O adapter; and
- which synchronous invariants the async layer deliberately delegates instead of reimplementing.

You now have a small expression framework whose type families, borrowed column views, scalar
vectorization, runtime erasure, logical binding, primitive specialization, Rust worker boundaries,
and optional batch async boundary remain separate and testable.

{{#include copyright.md}}
