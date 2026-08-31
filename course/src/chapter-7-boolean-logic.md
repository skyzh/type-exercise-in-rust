{{#include wip-banner.md}}

# Chapter 7: Select Dense Fixed-Width Loops

Chapter 6 gave strict scalar functions one reusable evaluator. Its general loop must support
nullable arrays, constants, typed nulls, and Indexed views, so every row asks each typed view for
an `Option`. A fixed-width array already stores raw values and a packed validity bitmap. We can
select that representation once per batch, run a simple typed value loop, and compose validity
separately.

This chapter specializes only strict, total, infallible `i32` operations. Checked division and
other fallible functions must keep the Chapter 6 loop so an invalid raw lane cannot manufacture an
error. SQL Boolean logic has its own non-strict null semantics in Chapter 8. Indexed columns keep
the general gather loop because their keys change which source row is read.

## What is in the starter

Begin from completed Chapter 6 and copy the first cumulative checkpoint:

```console
cargo x copy-test --chapter 7 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
```

The representation observation and traversal belong in the core package. The concrete `I32Add`
operation remains in the expression facade. You will add:

- a crate-private raw view over an `i32` array's values and validity;
- a raw value/validity representation for value and typed-null constants;
- explicit detection of Indexed inputs;
- four raw input-shape loops; and
- word-wise validity composition plus raw-result construction.

No raw representation becomes public. The existing public view constructors and error messages
stay unchanged.

## Checkpoint 1: bind the physical representation

Add a crate-private observation with these two shapes:

```rust,ignore
enum RawI32Column<'a> {
    Array {
        values: &'a [i32],
        validity: &'a BitVec,
    },
    Constant {
        value: i32,
        valid: bool,
        len: usize,
    },
}
```

`ColumnViewImpl::as_raw_i32` returns an array even when its validity contains nulls. A value
constant carries `valid = true`; a typed-null `i32` constant uses a harmless raw zero with
`valid = false`. Non-`i32` and Indexed inputs return `None`. Keep a separate crate-private
`is_indexed` observation so the evaluator can route Indexed inputs before matching raw shapes.

Run the first checkpoint again. Its two structural tests pin the exact private representation and
ensure Indexed detection remains separate. This checkpoint adds no public nullability proof or
second array type.

## Checkpoint 2: compute values and validity separately

Copy the second stage:

```console
cargo x copy-test --chapter 7 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
```

Implement `PrimitiveBinaryExpression<F>` for a strict, total
`BinaryScalarFunction<Left = i32, Right = i32, Output = i32>`. Validate arity, physical types, and
lengths once. Route Indexed inputs to `evaluate_binary` and report `PrimitiveLoop::Indexed`.
Otherwise match the two raw inputs once and choose array/array, array/constant, constant/array, or
constant/constant.

Each raw helper computes only values and is source-equivalent to this traversal:

```rust,ignore
for row in 0..len {
    output.push(function(left[row], right[row]));
}
```

Do not call `get`, inspect validity, match an operation, or construct `Option` inside those loops.
For constant/constant, call the scalar function once for a non-empty batch and fill the owned raw
result; an empty batch calls it zero times.

After values are computed, combine the two validity sources by `BitVec` storage word and truncate
the final word to the exact row count. A valid constant behaves like virtual all-ones; a typed-null
constant behaves like virtual all-zeros. Build the output through `I32Array::from_raw_parts`.

The four cumulative tests prove all raw shapes, nullable arrays across multiple storage words, the
independent validity result, and explicit Indexed fallback.

## Checkpoint 3: prove the safety boundaries

Copy the completed Chapter 7 test:

```console
cargo x copy-test --chapter 7 --checkpoint 3
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
cargo check -p type-exercise-starter-core --locked
```

The seven focused tests now cover all raw shapes, word-wise validity, one-call constant filling,
Indexed gathering, non-commutative operand order, unchanged arity/type/length errors, and the
fallible safety boundary. An invalid zero divisor must remain unobserved by checked division, while
the same zero under a valid bit still reports the existing row error.

Install `cargo-expand` if needed, then inspect the selected implementation:

```console
cargo expand -p type-exercise-starter-expr --lib numeric
```

The expanded facade contains only operation selection and scalar callbacks. All batch traversal is
in core. In `raw_array_array`, `raw_array_constant`, and `raw_constant_array`, confirm that the hot
loop performs typed loads, one preselected scalar call, and a push. This is a source-level
ownership guarantee, not a promise that LLVM vectorizes every target identically.

## Why the general loop stays

The strict raw loop is deliberately narrow. General `ColumnView::get` must choose whether a row is
null, Indexed views must gather through keys, fallible operations must not observe invalid raw
lanes, and SQL Boolean operators have non-strict three-valued semantics. Selecting the raw route
once gives the total fixed-width case an honest simple loop without disguising those required
branches elsewhere.

Next: [Chapter 8 adds three-valued Boolean logic](./chapter-8-runtime-erasure.md).

{{#include copyright.md}}
