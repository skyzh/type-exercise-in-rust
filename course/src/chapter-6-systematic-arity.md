{{#include wip-banner.md}}

# Chapter 6: Separate Fast Paths from Semantic Exceptions

The Chapter 5 adapters are multi-type: they build the output family selected by their generic
parameters and work for fixed-width scalars beyond Int32. An Int32-only raw-values lane can still
remove `Option` construction for the most common strict operations, but it must not become the
general evaluator.

```console
cargo x copy-test --chapter 6
cargo test -p type-exercise-starter-supplied-tests chapter_6 --locked
```

## Bound the raw Int32 lane

Add a crate-private `RawI32Column` view for Int32 Array and Constant inputs. Keep Indexed out of
that representation. Then implement public `auto_vectorize_primitive_i32` in core:

1. validate two Int32 inputs and equal lengths;
2. send any Indexed input to `auto_vectorize_binary`;
3. select Array/Array, Array/Constant, Constant/Array, or Constant/Constant once;
4. compute raw values with the supplied total operation; and
5. combine validity for the whole batch before constructing the output array.

The function accepts only `Fn(i32, i32) -> i32`. That signature is the safety boundary: wrapping
add, subtract, and multiply fit it; division does not.

## Keep exceptions and SQL nulls visible

Implement `try_evaluate_binary` and the fallible ternary adapter so the first scalar callback
failure names its row and returns no partial array. The supplied test uses checked integer division
to exercise zero without making core choose an arithmetic operator. Decimal remains unsupported
until precision, scale, rounding, and overflow have an explicit policy.

Boolean AND and OR are not strict functions. Add nullable-aware core adapters that pass
`Option<bool>` values to one supplied callback. The test expresses the SQL cases: `FALSE AND NULL`
is false, while the remaining unknown combinations stay null. Do not enable a concrete Boolean
facade here; Chapter 8 introduces NOT, AND, OR, and their physical factories together.

```console
cargo test -p type-exercise-starter-supplied-tests chapter_6 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
```

The tests cover wrapping Int32 Array/Constant shapes, Indexed fallback, first-error reporting, and
nullable Boolean behavior. [Chapter 7](./chapter-7-boolean-logic.md) places these complete batch
kernels behind one runtime-erased expression shell.

{{#include copyright.md}}
