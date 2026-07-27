# Rust Language Concepts by Task

This appendix is a map from advanced Rust concepts to the database-framework problem that makes
each concept necessary. It does not add a parallel feature-tour curriculum. Complete the linked
framework task first, then use the concept name to explain why the resulting API has its shape.

## Learning Contract

**Starting state:** Complete Part II through
[Bind and Execute the Complete Framework](../vectorized/framework.md), or begin each row below from
the chapter named in its **Exact task** column. The workspace crates are the source of truth; the
historical `archive/` snapshots intentionally use older APIs.

**Goal:** For every feature, connect a compiler-enforced relationship to one observable database
capability and explain one boundary or failure case.

**Non-goals:** Do not replace the physical layouts, put dynamic dispatch inside row loops, add an
async runtime, perform one asynchronous request per row, or introduce unsafe pinning. Performance
numbers remain observations rather than correctness gates.

For implementation work, preserve public behavior and protected tests. Required behavior and
invariants below are normative; snippets show one compatible design rather than replacing the
contract.

## Task Map

| Rust concept | Database pressure | Exact task | Completion evidence |
| --- | --- | --- | --- |
| Lifetimes and GATs | Strings and lists borrow values from array buffers | [Implement array access and iteration](../vectorized/array.md#task), then [trace owned and borrowed scalars](../vectorized/scalar.md#task) | Explain why `RefItem<'a>` and `RefType<'a>` cannot outlive the source array |
| HRTBs and associated-type equality | A generic kernel must accept borrowed values for every batch lifetime | [Trace the scalar equality constraints](../vectorized/scalar.md#task) | Identify which relationship must hold `for<'a>` rather than for one caller-selected lifetime |
| Generics and RPITIT | All physical arrays share one iterator without exposing its concrete type | [Implement the default `Array::iter`](../vectorized/array.md#task) | Empty, null-containing, primitive, and string arrays use the same iterator contract |
| Covariance | A long-lived borrowed column view must be usable in a shorter evaluation scope | [Compile the lifetime-shortening check](../vectorized/column_view.md#task) | Explain why no runtime conversion occurs, and distinguish reference-lifetime covariance from `&mut T`'s invariance over `T` |
| Dyn compatibility, trait upcasting, and downcasting | Runtime storage sometimes needs an open erased container | [Modernize `DynArray` erasure](../vectorized/impls.md#task) | Owned and borrowed `BoxedArray` round trips pass without `as_any`/`into_any` shims |
| `Fn`, `PhantomData`, and auto traits | A typed scalar kernel is reused across batches and executor threads | [Inspect the generated function template](../vectorized/func.md#task) | Explain why `FnOnce` and removing `PhantomData` violate different compile-time requirements |
| Declarative macros and generated syntax | Numeric promotion is a dense policy matrix; custom functions are not | [Trace one numeric and one custom expression](../vectorized/data_types.md#task) | Locate the policy table, callback macro, concrete expression type, and custom escape hatch |
| `Send` and `Sync` | A bound expression may move between workers or be shared by them | [Check the executor boundary](../vectorized/framework.md#task) | `Box<dyn Expression>` satisfies both auto traits; explain `Rc` versus `Arc` captures |
| `Future`, `Pin`, `Unpin`, and static versus erased futures | An external UDF may wait once per batch without changing synchronous row kernels | [Add a batch asynchronous boundary](#optional-task-add-a-batch-asynchronous-boundary) | Both static and erased ready futures return one array; a non-array input returns the specified error |

## Why These Features Belong Together

The framework crosses two different boundaries:

```text
borrowed, generic, monomorphized code
              |
              | erase at a batch boundary
              v
runtime enums or dyn-compatible traits
```

Lifetimes, GATs, HRTBs, covariance, RPITIT, and `PhantomData` describe the static side. Trait
objects, upcasting, downcasting, and auto traits describe what must remain true after erasure.
Macros manufacture repeated static combinations but do not change either boundary.

This distinction also explains why the course has two erasure strategies. `ArrayImpl` is an
exhaustive enum for the closed set of core physical layouts. `DynArray` is an open trait-object
boundary used by recursively nested arrays. The task is to understand the tradeoff, not to replace
one with the other everywhere.

## Closure Capabilities Are an API Decision

`Fn`, `FnMut`, and `FnOnce` describe how a callable may use its captures; they are not performance
levels. The generated expression receives `&self` and may evaluate repeatedly, so `F: Fn` is the
narrow capability that matches the API. `Send + Sync` is a separate requirement imposed by the
executor boundary.

Before checking the compiler, predict these cases:

1. a function item with no captures;
2. a closure that reads an immutable `Arc<String>`;
3. a closure that mutates a captured counter directly; and
4. a closure that moves a captured `String` out on its first call.

Classify them as `Fn`, `FnMut`, or `FnOnce`, then state which could be stored in a reusable
`Expression`. A synchronized stateful kernel remains possible, but it should make that state and
its contention explicit instead of weakening every generated scalar function.

## `PhantomData` and Variance Have Concrete Effects

`BinaryExpression<I1, I2, O, F>` stores `F` but no value of the three scalar types. Its
`PhantomData<(I1, I2, O)>` field makes those types part of the struct's static model without adding
runtime bytes. This influences auto traits, variance, and drop checking. It does not allocate and
is not a substitute for a borrowed lifetime field.

By contrast, `ColumnViewImpl<'a>` contains real shared references, so the compiler derives
covariance over `'a`. The linked task proves only this concrete lifetime-shortening relationship.
Do not infer the variance of an arbitrary GAT projection from its syntax; the associated type's
implementation determines how its parameters are used.

## Optional Task: Add a Batch Asynchronous Boundary

**Task ID:** `rust-async-batch-boundary`

**Before:** `Expression::eval` returns one materialized `ArrayImpl` synchronously. Its per-row loops
contain no suspension points.

**After:** The example can represent either a statically known future or a dyn-compatible boxed
future for one whole batch. The synchronous framework remains unchanged.

**Relevant file:** `expr-common/examples/async_expression.rs`

### Behavioral Contract

1. `eval_static` returns `impl Future<Output = Result<ArrayImpl>> + Send + 'a`; the function chooses
   one hidden future type and captures the input borrow for no longer than `'a`.
2. `AsyncExpression::eval_async` returns this erased type:

   ```rust,ignore
   type BatchFuture<'a> =
       Pin<Box<dyn Future<Output = Result<ArrayImpl>> + Send + 'a>>;
   ```

3. The future represents one batch. No input reference or scalar borrowed from that batch may
   escape the future's lifetime.
4. A regular one-column array input produces an equivalent output array through both paths.
5. Any non-array view returns `expected one regular array input`; a wrong input count returns
   `expected one input`.
6. No async runtime, network service, unsafe code, or per-row future is introduced.

The identity operation is intentionally small: it tests the boundary rather than pretending to be
a production remote UDF.

### Checkpoints

1. **Static future:** Implement or inspect `eval_static`. Identify the concrete function that
   chooses the hidden future type and the caller that remains generic over it.
2. **Erased future:** Implement or inspect `BatchFuture`, `AsyncExpression`, and `AsyncAdapter<E>`.
   Explain why the box and vtable are paid once per batch.
3. **Pinning:** Poll the known-ready future in the test only after pinning it. `Future::poll`
   receives `Pin<&mut Self>` because some future state machines may contain address-sensitive
   internal references.
4. **`Unpin`:** Explain why `Pin<Box<dyn Future<...>>>` can move as a pointer value while its
   pointee remains at a stable address. The example does not require the erased future itself to
   implement `Unpin`.
5. **Boundary case:** Pass a constant view and assert the specified error. Passing tests for only
   the regular-array case is incomplete evidence.

Run the canonical check from the repository root:

```console
cargo test -p expr-common --example async_expression --locked
```

Expected result: three tests pass—one normal case through both future representations, one invalid
view case, and one wrong-arity case. The check requires only the pinned Rust toolchain and workspace
dependencies and should finish in seconds after the workspace is built.

Stop when the boundary and tests compile. A real network client, cancellation policy, retry logic,
timeouts, runtime selection, streaming output, and executor scheduling belong to a separate
external-UDF course. In particular, do not move `.await` into the generated row loop.

### Teach Back

Before considering the task complete, answer:

- Why can `eval_static` return an unboxed opaque future while `dyn AsyncExpression` returns a boxed
  one?
- Why would a trait method returning RPIT or using `async fn` directly not be dyn-compatible?
- What exactly is pinned: the `Box` handle, its allocation, or both?
- Which borrow prevents the batch inputs from being freed while the future can still read them?
- Where would cancellation and remote error semantics need to be specified before this became a
  production feature?

## Course Validation

After changing any linked implementation task, run:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
mdbook build tutorial
```

Report commands actually run, skipped checks, known limitations, and whether the work was
agent-generated, human-skimmed, understood, or independently reproduced. Passing tests establish
executable evidence; they do not establish human understanding.

## Authoritative References

- [Impl Trait and Edition 2024 capture rules](https://doc.rust-lang.org/stable/reference/types/impl-trait.html)
- [Higher-ranked trait bounds](https://doc.rust-lang.org/stable/reference/trait-bounds.html#higher-ranked-trait-bounds)
- [Subtyping and variance](https://doc.rust-lang.org/stable/reference/subtyping.html)
- [Dyn compatibility](https://doc.rust-lang.org/stable/reference/items/traits.html#dyn-compatibility)
- [Trait upcasting stabilization in Rust 1.86](https://blog.rust-lang.org/2025/04/03/Rust-1.86.0/)
- [`Future`](https://doc.rust-lang.org/stable/std/future/trait.Future.html)
- [`Pin` and `Unpin`](https://doc.rust-lang.org/stable/std/pin/index.html)
- [`PhantomData`](https://doc.rust-lang.org/stable/std/marker/struct.PhantomData.html)
- [`Send` and `Sync`](https://doc.rust-lang.org/stable/nomicon/send-and-sync.html)

The Rust Reference and standard-library documentation define current behavior. The Rustonomicon is
useful for deeper unsafe-code motivation, but it is not a checklist requiring unsafe machinery in
this framework.

{{#include ../copyright.md}}
