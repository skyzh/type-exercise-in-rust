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

- a public `PrimitiveBinaryExpression` boundary whose ordinary `evaluate` method returns results;
- a crate-private raw view over an `i32` array's values and validity;
- a raw value/validity representation for value and typed-null constants;
- explicit detection of Indexed inputs;
- four raw input-shape loops; and
- word-wise validity composition plus raw-result construction.

No raw representation becomes public. The existing public view constructors and error messages
stay unchanged.

## Checkpoint 1: establish the public evaluation boundary

Add `PrimitiveBinaryExpression<F>` for a strict, total
`BinaryScalarFunction<Left = i32, Right = i32, Output = i32>`. Validate arity, physical types, and
lengths once. For this checkpoint, its public `evaluate` method can delegate every input shape to
the Chapter 6 `evaluate_binary` traversal. That baseline is already correct for arrays, constants,
typed nulls, and Indexed inputs; the next checkpoint changes how dense inputs are traversed.

Run the first checkpoint again. Before the new public type exists, both tests fail to compile. Once
it evaluates dense, typed-null, and Indexed inputs through ordinary public results, the cumulative
suite has 48 passing tests. Keep the representation choices private; only the evaluation result
belongs to the public API.

## Checkpoint 2: bind the physical representation

Now add a crate-private observation with these two shapes:

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

Copy the second stage:

```console
cargo x copy-test --chapter 7 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
```

Route Indexed inputs to `evaluate_binary`. Otherwise match the two raw inputs once and choose
array/array, array/constant, constant/array, or constant/constant. Keep the raw representation
crate-private; the existing loop diagnostic may report which route was selected, but supplied
tests use only the public `evaluate` result.

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

The first two tests retain the public evaluation boundary. Two more cases exercise nullable arrays
across multiple storage words and require constant/constant evaluation to call the scalar function
once for a non-empty batch and zero times for an empty batch. After the dense path is connected,
the cumulative suite has 50 passing tests. The raw binding and loop selection remain private
implementation details.

## Checkpoint 3: prove the safety boundaries

Copy the completed Chapter 7 test:

```console
cargo x copy-test --chapter 7 --checkpoint 3
cargo test -p type-exercise-starter-supplied-tests chapter_7 --locked
cargo check -p type-exercise-starter-core --locked
```

The seven focused tests now cover dense and Indexed results, word-wise validity, one-call constant
filling, non-commutative operand order, unchanged arity/type/length errors, and the fallible safety
boundary. An invalid zero divisor must remain unobserved by checked division, while the same zero
under a valid bit still reports the existing row error. The cumulative suite has 53 passing tests.

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

[Chapter 8 completes this module with three-valued Boolean
logic](./chapter-8-runtime-erasure.md), where null is part of the scalar semantics rather than a
reason to skip the operation.

{{#include copyright.md}}
