# Strengthen Rust Type Boundaries

Chapter 6 preserved runtime behavior while selecting faster loops. This chapter preserves the
same results, errors, null propagation, binding, and loop selection while moving several
assumptions into Rust's type system.

The framework already relies on four facts:

- array iteration returns values tied to the array borrow;
- an erased expression can be recovered only through a checked runtime type test;
- expressions and logical factories can be shared with executor workers; and
- a column view may use a shorter borrow than the data it references.

You will make each fact visible in a public signature or compile-time check. No row kernel or
logical signature changes in this chapter.

## Starting Point and Public Pieces

Continue from your completed Chapter 6 implementation. Keep its behavior and public names except
for the deliberately hidden iterator type, and add or strengthen these boundaries:

- `Array::iter` returns an opaque iterator whose items borrow for the same lifetime as `&self`;
- `Expression` has `Any + Send + Sync` supertraits;
- `BinaryScalarFunction` has `Send + Sync + 'static` supertraits;
- stored logical factories and `FunctionRegistry::register_binary` require `Send + Sync + 'static`;
- the concrete `ColumnViewImpl<'a>` representation remains covariant over `'a`; and
- the concrete array iterator becomes an implementation detail rather than a public return type.

Copy the Chapter 7 tests before implementing:

```console
cargo x copy-test --chapter 7
cargo test -p type-exercise-starter chapter_7 --locked
```

That first focused run should fail to compile because the starter does not yet satisfy the new
type boundaries. A successful run reporting zero matched tests means the Chapter 7 contract was
not copied. After the implementation is complete, the same focused command passes six tests.

The rolling stable toolchain selected by this repository supports return-position `impl Trait` in
traits and direct trait-object upcasting. The repository still declares no older minimum supported
Rust version.

## Hide the Iterator Without Hiding Its Borrow

Every array already implements `get` and `len`, so the `Array` trait can supply iteration once. Use
return-position `impl Trait` in the trait, often shortened to RPITIT:

```rust,ignore
fn iter<'a>(&'a self) -> impl Iterator<Item = Option<Self::RefItem<'a>>> + 'a {
    ArrayIterator::new(self)
}
```

The return type hides `ArrayIterator<'a, Self>` from callers. Its item type still exposes the
important relationship: a borrowed string item is valid for `'a`, the same lifetime as the array
borrow. Hiding the iterator does not erase item lifetimes.

`Array` was already not dyn-compatible because it requires `Sized` and uses generic associated
types. Returning an opaque iterator therefore does not remove an existing `dyn Array` use. It
narrows the public contract to the behavior callers need: an iterator over nullable borrowed
values.

Make the hidden concrete type part of the learner-owned compile-time contract. Add a compile-fail
doctest beside `Array::iter` in the starter crate:

````rust,ignore
/// ```compile_fail
/// use type_exercise_starter::ArrayIterator;
/// ```
````

The doctest names the starter crate intentionally. It passes only when callers cannot import the
concrete iterator; `cargo test -p type-exercise-starter --doc --locked` runs this privacy check.

Verify both ends of the behavior. An empty array ends immediately, while nullable integer and
string arrays yield their values and nulls in order. For strings, collect `Option<&str>` rather
than allocating owned strings; this proves the iterator retains the Chapter 1 borrow relationship.

## Upcast an Existing Trait Object

The physical catalog already returns `Box<dyn Expression>`. Add `Any` as a supertrait so a caller
that genuinely needs the concrete type can upcast the borrowed trait object directly:

```rust,ignore
let expression = build_builtin_expression("i32_add").unwrap();
let erased: &dyn Any = expression.as_ref();
let add = erased.downcast_ref::<PrimitiveBinaryExpression<I32Add>>();
```

The upcast itself does not prove which concrete expression is stored. `downcast_ref` remains a
checked operation: the matching type returns `Some`, and a wrong type returns `None`. Keep runtime
metadata and `evaluate_with_loop` as the normal behavior-facing interfaces. Downcasting is a
narrow recovery tool, not a replacement for the Chapter 4 object-safe contract.

`Any` also means concrete expressions are `'static`: their types cannot contain non-static
borrows. That fits this catalog, whose expressions own their scalar-function values and are stored
for reuse across batches.

## State the Worker Contract

The erased expression and logical registry may be moved to or shared with executor workers. Make
that requirement explicit:

```rust,ignore
pub trait Expression: Any + Send + Sync {
    // existing metadata, evaluation, and observer methods
}
```

An expression stores its scalar-function value, so `BinaryScalarFunction` must carry the same
`Send + Sync` requirement. Its `'static` bound closes the `Any` boundary for generic expression
adapters.

The logical registry stores boxed factory closures. Add `Send + Sync + 'static` to both the erased
factory type and the closure accepted by `register_binary`. `Fn` remains the correct call trait:
one registered factory can serve many binding requests through `&self`. `FnMut` would require
exclusive or internally synchronized mutation, while `FnOnce` could be called only once.

A closure may still capture state. An `Arc<AtomicUsize>`, for example, is compatible because the
capture is itself thread-safe. A closure that captures `Rc` should fail at registration rather
than making the entire registry unexpectedly unsafe to share.

## Prove Safe Lifetime Shortening

`ColumnViewImpl<'a>` contains shared references to arrays, dictionary keys, or borrowed scalar
values. A view with a longer borrow may be used where a shorter borrow is required:

```rust,ignore
fn shorten<'short, 'long: 'short>(
    view: ColumnViewImpl<'long>,
) -> ColumnViewImpl<'short> {
    view
}
```

This compiles because the concrete enum is covariant over `'a`: every stored use of the lifetime
permits safe shortening. The function cannot extend a borrow, and the test does not claim that all
generic types are covariant. Variance depends on how each lifetime appears in the concrete type.

No `unsafe` code, transmute, leaked allocation, or forged `'static` reference belongs here. The
compiler already proves the allowed direction.

## The Chapter Contract

Preserve these rules:

1. Opaque array iteration yields the same nullable rows as indexed access, including borrowed
   strings and empty arrays, without exposing the concrete iterator type.
2. Direct `dyn Expression` to `dyn Any` upcasting is followed by a checked downcast.
3. `Box<dyn Expression>` and `FunctionRegistry` are both `Send + Sync`.
4. Scalar-function values and registered factories satisfy the same worker boundary.
5. `ColumnViewImpl<'long>` may be shortened only when `'long: 'short`.
6. Chapter 1–6 values, errors, binding, null behavior, and primitive-loop selection remain
   unchanged.

## Implementation Checkpoints

1. Change `Array::iter` to return `impl Iterator` with an explicit borrow lifetime.
2. Stop exporting the concrete array iterator as part of the public API and add its compile-fail
   doctest beside `Array::iter`.
3. Add `Any + Send + Sync` to `Expression` and close the generic function lifetime boundary.
4. Require stored logical factories and registration closures to be `Send + Sync + 'static`.
5. Exercise matching and mismatching checked expression downcasts.
6. Add worker-thread tests for erased expressions and a registry with thread-safe captured state.
7. Add the concrete lifetime-shortening compilation test and keep all earlier chapters green.

## Review Your Chapter Result

Run:

```console
cargo test -p type-exercise-starter chapter_7 --locked
cargo test -p type-exercise-starter --lib --locked
cargo test -p type-exercise-starter --doc --locked
```

The Chapter 7 contract contains six supplied tests plus one compile-fail doctest. The supplied
tests cover empty and nullable opaque iteration, borrowed string items, direct trait-object
upcasting with checked recovery, worker-safe erased expressions, thread-safe captured factory
state, and concrete column-view covariance. The doctest separately proves that the concrete
iterator cannot be named through the public API.

Before continuing, explain:

- what RPITIT hides and which lifetime relationship remains visible;
- why trait-object upcasting does not replace checked downcasting;
- why `Any` adds a `'static` boundary to concrete expressions;
- why reusable factories use `Fn + Send + Sync`; and
- what the covariance test proves, and what it does not prove.

Chapter 8 will keep the synchronous row loop and add one asynchronous boundary around a whole
batch.

{{#include copyright.md}}
