# Day 4: Strengthen Rust Type Boundaries

The first three days established observable behavior: borrowed views cannot outlive their arrays,
boxed arrays can recover their concrete representation, expressions can run on executor threads,
and every array can be iterated without changing its value semantics.

Today you will make those properties explicit in the Rust interfaces. The result is less
compatibility plumbing and more compile-time evidence at the boundaries where the framework
already relies on the behavior.

## Starting Point and Result

Day 3 has a complete synchronous expression path. It still repeats one iterator method in every
array implementation, converts `DynArray` through historical `as_any` helpers, and leaves
expression thread safety implicit in its concrete types.

After this day:

- `Array::iter` supplies one opaque default iterator for every array;
- `DynArray: Any` upcasts directly to `dyn Any` for checked owned and borrowed downcasts;
- `Expression: Send + Sync` states the executor's thread-safety requirement;
- generated evaluator closures carry the same `Send + Sync` contract; and
- a compile-time test proves that `ColumnViewImpl<'long>` can be used for a shorter borrow.

The workspace minimum becomes Rust 1.86, the first stable release with trait-object upcasting.
The pinned Rust 1.94 toolchain remains the recommended environment.

This day does not make the evaluator asynchronous. Day 5 will add one explicit batch-level async
adapter without changing the synchronous row loop.

## One Iterator, Including Borrowed Strings

Every array already has `get` and `len`. The old interface required each implementation to repeat:

```rust,ignore
fn iter(&self) -> ArrayIterator<'_, Self> {
    ArrayIterator::new(self)
}
```

Move that implementation to the trait:

```rust,ignore
fn iter<'a>(
    &'a self,
) -> impl Iterator<Item = Option<Self::RefItem<'a>>> + 'a {
    ArrayIterator::new(self)
}
```

The observable behavior comes first: iterating `[Some("db"), None, Some("rust")]` yields borrowed
`&str` values and one null in the same order; iterating an empty array ends immediately. The
return-position `impl Trait` in a trait, commonly called RPITIT, hides the concrete
`ArrayIterator<'a, Self>` while preserving that every borrowed item is tied to the array borrow.

`Array` is already not dyn-compatible because it requires `Sized` and contains generic associated
types. The opaque iterator therefore does not remove a trait-object use that the interface
previously supported.

## Remove Historical `Any` Shims

`BoxedArray` stores `Box<dyn DynArray>`. To recover an `ArrayImpl`, it reads the physical type and
performs a checked downcast to the matching array:

```text
Box<dyn DynArray>
        |
        | direct upcast
        v
Box<dyn Any>
        |
        | checked downcast using PhysicalType
        v
Box<I32Array> or another concrete array
```

Because `DynArray: Any`, Rust 1.86 can coerce the trait object directly:

```rust,ignore
let owned: Box<dyn Any> = boxed_dyn_array;
let borrowed: &dyn Any = &*boxed_dyn_array;
```

Delete `into_any` and `as_any`; they restate a relationship already present in the supertrait.
Keep both the `PhysicalType` dispatch and checked `downcast` calls. An enum remains the closed,
exhaustive runtime representation, while `Any` provides the open type-erased boundary.

## State the Thread Boundary

The registry stores expressions that may be shared across executor workers. Make that requirement
part of the erased interface:

```rust,ignore
pub trait Expression: Send + Sync {
    fn eval(&self, inputs: &[ColumnViewImpl<'_>]) -> anyhow::Result<ArrayImpl>;
}
```

Generated expressions store a closure `F`, so `F` must also be `Send + Sync`. The evaluator uses
`Fn` because one bound expression can evaluate many batches through `&self`.

- `FnMut` would require mutable or internally synchronized state for repeated evaluation.
- `FnOnce` could consume captured state and would not support a second batch.

`PhantomData<(I1, I2, O)>` remains in the generated struct because those scalar types determine its
type identity and auto-trait behavior even though the struct stores no values of those types.

## Prove Lifetime Shortening

A Day 1 view only borrows arrays, scalars, or dictionary keys. Code that holds
`ColumnViewImpl<'long>` should be able to use it where a shorter lifetime is required:

```rust,ignore
fn shorten<'short, 'long: 'short>(
    view: ColumnViewImpl<'long>,
) -> ColumnViewImpl<'short> {
    view
}
```

This compiles because the enum is covariant over its borrow lifetime: every stored occurrence of
`'a` is in a position where shortening is safe. Do not generalize that statement to every generic
associated type or every user-defined lifetime parameter. The test proves this concrete view.

## The Type-Boundary Contract

Implement and preserve these rules:

1. `Array::iter` yields the same nullable sequence as indexed `get` for empty, primitive, string,
   decimal, Boolean, and list arrays.
2. A borrowed iterator item cannot outlive the array borrow.
3. Converting `BoxedArray` to owned or borrowed `ArrayImpl` preserves its physical type, length,
   values, and nulls.
4. Runtime downcasts remain checked; a `PhysicalType` mismatch must not become unchecked memory
   access.
5. Every `Box<dyn Expression>` is `Send + Sync`, including its stored closure.
6. `ColumnViewImpl` permits safe lifetime shortening but never lifetime extension.
7. The Day 1–3 data flow, errors, null propagation, and fast-path selection remain unchanged.

Using RPITIT, direct trait upcasting, and explicit auto-trait bounds is the selected course design.
Local test organization and helper names remain implementation choices.

## Implementation Checkpoints

Work in this order:

1. Move the iterator implementation into `Array` and delete the repeated concrete methods.
2. Add empty and nullable iterator tests.
3. Remove `DynArray::{into_any, as_any}` and use direct owned and borrowed upcasts.
4. Extend the boxed-array test through both borrowed and owned downcasts.
5. Add `Send + Sync` to `Expression` and to stored generated closures.
6. Add the covariance compilation test for `ColumnViewImpl`.

Keep changes inside the workspace minimum-version declaration, array and expression interfaces,
column-view tests, and generated closure bounds. Do not add an async runtime or change kernel
semantics.

## Verify the Day

Run:

```console
cargo test -p expr-common --locked
cargo test -p expr-template --locked
cargo test -p expr-impl --locked
```

The tests should exercise empty and nullable iteration, borrowed and owned downcasts, view lifetime
shortening, erased expression auto traits, and all previous expression behavior.

Before moving on, explain:

- what the opaque iterator hides and which lifetime relationship it preserves;
- why direct trait upcasting replaces helpers but not checked downcasting;
- why a reusable expression stores `Fn` rather than `FnOnce`; and
- what the covariance test proves—and what it does not prove.

Next, you will add an asynchronous boundary around whole batches.

{{#include copyright.md}}
