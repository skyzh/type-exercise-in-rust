{{#include wip-banner.md}}

# Chapter 8: Erase Typed Expressions at Runtime

The engine now has typed kernels, but a runtime function name cannot carry a Rust generic parameter.
This chapter places the typed shells behind one object-safe `Expression` interface.

**Prerequisites:** Chapters 6-7, trait objects, and checked enum recovery.

**By the end of this chapter, you will:**

- expose name, input types, output type, and evaluation through `dyn Expression`;
- select builtin physical expressions from one catalog; and
- preserve the typed evaluator's arity, type, length, null, and contextual operation errors.

```console
cargo x copy-test --chapter 8
cargo test -p type-exercise-starter chapter_8 --locked
```

The first run should fail on the object-safe expression boundary or physical catalog.

## Keep erasure outside the row loop

The runtime path is:

```text
physical name → Box<dyn Expression> → typed whole-batch kernel → typed row loop
```

The object erases one already-vectorized evaluator. Its stored batch-kernel pointer selects the
typed implementation once; that implementation validates the batch, converts columns to typed
views, and enters the row loop. It must not erase a scalar callback or match on `ScalarRefImpl` to
select an operator for every row.

## Checkpoint 1: make the batch contract safe to erase

- **Target:** `type-exercise-starter/src/expression.rs::{Expression, BinaryExpression, BinaryBatchKernel}`.
- **Change:** keep `name`, `arity`, `input_types`, `output_type`, and `evaluate` free of associated
  types, and require `Any + Send + Sync` for checked recovery and sharing, so the trait is
  object-safe from this chapter on; `BinaryExpression::new` pairs runtime physical metadata with
  one whole-batch kernel pointer.
- **Preserve:** metadata is borrowed or copied from the selected expression; runtime inputs stay
  borrowed.
- **Run:** the Chapter 8 focused test.
- **Passing means:** a builtin evaluates through `Box<dyn Expression>` with the same result as its
  typed adapter.

Wire the erased boundary and catalog into the starter crate root like the earlier chapters:

```rust,ignore
pub use expression::{
    BinaryBatchKernel, BinaryExpression, BUILTIN_EXPRESSION_NAMES, Expression,
    build_builtin_expression,
};
```

## Checkpoint 2: erase the fixed-arity batch shell

- **Target:** the `Expression` implementation for `BatchExpression<N>` in
  `type-exercise-starter/src/operators.rs`, plus the original
  `BinaryExpression` implementation in `type-exercise-starter/src/expression.rs` and the
  transactional string builder in `type-exercise-starter/src/array/string_array.rs`.
- **Change:** publish physical metadata and call the selected whole-batch kernel. One declarative
  catalog row owns each built-in's name, input and output physical types, kernel, and optional loop
  specialization; that same row list generates both the public name list and constructor lookup.
  The typed `i32_add` and `string_concat` kernels own their row loops; `BinaryExpression` never
  stores or invokes a scalar callback. String concatenation writes both borrowed fragments directly
  into the final byte buffer through `StringValueWriter` and
  `StringArrayBuilder::try_push_with`; `push_null` publishes null rows. A failed closure truncates
  any bytes it appended before the builder publishes an offset or validity bit. The erased boundary
  also checks that the returned array's physical type matches the declared output type.
- **Preserve:** arity is checked before indexing; type and length messages retain their context;
  strict nulls skip the write closure; failed variable-width rows leave no partial bytes or metadata.
- **Run:** focused and cumulative tests.
- **Passing means:** erasure adds selection, not a second evaluator.

## Checkpoint 3: build the physical catalog

- **Target:** `type-exercise-starter/src/expression.rs::{define_builtin_expressions, build_builtin_expression,
  BUILTIN_EXPRESSION_NAMES}`.
- **Change:** make registered names and constructors one source of truth.
- **Preserve:** missing names return `None`; catalog metadata must match the actual expression.
- **Run:** the Chapter 8 catalog and delegation tests.
- **Passing means:** every listed physical builtin is constructible and no unlisted name succeeds.

## Required and extension work

Checked runtime erasure and a complete physical catalog are required. The vectorized kernels from
Chapters 4–6 keep the same batch behavior; this chapter changes how the engine selects them.
Dynamic plugin loading and per-row erased dispatch are extensions outside this course.

```console
cargo test -p type-exercise-starter chapter_8 --locked
cargo test -p type-exercise-starter --lib --locked
```


Next: [Chapter 9 binds logical calls to one physical kernel](./chapter-9-binding-coercion.md).

{{#include copyright.md}}
