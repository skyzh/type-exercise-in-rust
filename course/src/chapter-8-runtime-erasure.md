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
physical name → Box<dyn Expression> → generated typed adapter → shared auto-vectorizer
```

The object erases one generated adapter. Its stored batch-kernel pointer selects the typed
implementation once; that adapter calls the shared Day 6 evaluator with an ordinary scalar
function. It must not store `dyn Fn` or match on `ScalarRefImpl` for every row.

## Checkpoint 1: make the batch contract safe to erase

- **Target:** `type-exercise-starter/src/expression.rs::{Expression, BinaryExpression}`.
- **Change:** add `Expression` with `name`, `arity`, `input_types`, `output_type`, and `evaluate`
  free of associated types, and require `Any + Send + Sync` for checked recovery and sharing.
  Implement it for the Day 5 `BinaryExpression`, whose constructor already pairs runtime physical
  metadata with one whole-batch kernel pointer; do not recreate that shell here.
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
  The `i32_add` adapter reuses Day 6's binary auto-vectorizer. Variable-width output adds a
  consumed typestate: a scalar string function receives `Writer<'a>`, calls `write(self, ...)`
  exactly once, and returns `WriterUsed<'a>`. The evaluator alone recovers the builder for the next
  row. Zero writes cannot return `WriterUsed`; a second write cannot reuse the consumed `Writer`.
  The write closure may append both borrowed fragments directly through `StringValueWriter`, while
  `push_null` publishes strict null rows. The erased boundary also checks the returned physical type.
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

Checked runtime erasure and a complete physical catalog are required. The Day 6 auto-vectorizers
keep the same batch behavior; this chapter selects their adapters and adds exactly-once string output.
Dynamic plugin loading and per-row erased dispatch are extensions outside this course.

```console
cargo test -p type-exercise-starter chapter_8 --locked
cargo test -p type-exercise-starter --lib --locked
```


Next: [Chapter 9 binds logical calls to one physical kernel](./chapter-9-binding-coercion.md).

{{#include copyright.md}}
