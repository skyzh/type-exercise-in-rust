# Chapter 2: Read Arrays, Constants, and Dictionaries

Suppose a string function receives four logical rows:

```text
dictionary values: ["rust", null, "database"]
row keys:          [2,      null, 1,    0]
constant needle:   "a"
```

The left input should read as `["database", null, null, "rust"]`. The right input should read as
four copies of `"a"`. Neither input should be expanded into a temporary `StringArray`.

In this chapter, you will build a borrowed column view that exposes those logical rows through the
type family from Chapter 1.

## Starting Point and Result

Continue from your own completed Chapter 1 implementation. Its traits should enforce these two
families:

```text
i32     <-> i32    <-> I32Array
String  <-> &str   <-> StringArray
```

Before this chapter, a generic reader accepts only a regular typed array. After this chapter:

- `ColumnViewImpl<'a>` borrows a runtime-erased array, constant, or dictionary;
- `ColumnView<'a, S>` exposes `Option<S::RefType<'a>>` for all three encodings;
- a typed null retains its `PhysicalType` even though it has no scalar value;
- every non-null dictionary key is checked before a typed view is created; and
- a wrong physical type fails before any row is read.

The result is a reader, not an expression framework. Generated evaluators, input-length matching,
logical binding, macros, and fast paths remain later work.

Expect roughly two to three hours.

## Chapter Boundary

Copy the Chapter 2 test from the repository root before validation:

```console
cargo x copy-test --chapter 2
```

Then add `type-exercise-starter/src/column.rs` and export its public types from
`type-exercise-starter/src/lib.rs`. Keep the Chapter 1 public API and all copied tests unchanged.
Use public wrapper types whose fields and encoding variants remain private, so callers cannot
construct an unchecked dictionary. Do not add dependencies, expressions, generated code, or new
scalar types.

The three encodings, private representation, and validation rules are course decisions. Naming
private helpers is your choice. A correct implementation may match the typed view inside `get`;
moving representation dispatch outside a future hot loop belongs to the expression chapter.

## One Logical Column, Three Encodings

Begin with the behavior rather than the generic type. Every view answers the same three questions:

1. What physical value type do the rows contain?
2. How many logical rows are visible?
3. What nullable borrowed value appears at row `r`?

The answers come from different places:

| Encoding | Physical type | Logical length | Value at row `r` |
| --- | --- | --- | --- |
| Array | the borrowed array | array length | `array.get(r)` |
| Constant | an explicit type tag | repeat count | the same optional scalar |
| Dictionary | the values array | key count | `key[r]` selects `values` |

The erased boundary should have this shape:

```rust,ignore
pub struct ColumnViewImpl<'a> {
    kind: ColumnViewImplKind<'a>,
}

enum ColumnViewImplKind<'a> {
    Array(&'a ArrayImpl),
    Constant {
        value: Option<ScalarRefImpl<'a>>,
        physical_type: PhysicalType,
        len: usize,
    },
    Dictionary {
        indices: &'a [Option<usize>],
        values: &'a ArrayImpl,
    },
}
```

The wrapper is public, but `kind` and its variants are private. Callers can create views only
through `array`, `constant`, `null`, and the validating `dictionary` constructor. Each private
variant borrows its data. Copying a view may copy a reference, a small scalar, a type tag, and a
length; it must not clone array buffers or materialize logical rows.

## Why a Null Constant Needs a Type

Predict the physical type of this value:

```rust,ignore
let value = None;
```

There is no answer. `None` alone cannot distinguish a null integer from a null string. The
`Constant` variant therefore stores `physical_type` separately from its optional value. Provide two
constructors:

- `constant(value, len)` derives the physical type from a non-null `ScalarRefImpl`; and
- `null(physical_type, len)` receives the type explicitly.

The typed conversion checks that tag even when `len == 0`. An empty view still has a physical type
because planning and dispatch cannot infer one from its rows.

## Validate Dictionary Keys at the Boundary

A dictionary separates the row-to-value mapping from the values themselves:

```text
values: ["rust", null, "database"]
keys:   [2,      null, 1,    0]
rows:   ["database", null, null, "rust"]
```

A null key produces a null row without reading the values array. A non-null key `k` reads
`values[k]`; that value may independently be null.

`ColumnViewImpl::dictionary` must scan every non-null key. Reject the first key for which
`key >= values.len()` and return:

```rust,ignore
InvalidDictionaryKey {
    row,
    key,
    dictionary_len: values.len(),
}
```

Validate at construction rather than relying on a later indexing panic. Once construction
succeeds, `ColumnView::get` may use the key knowing it is in bounds. Null keys need no validation.

Work through both boundaries:

```text
values length 1, keys [0, 1] -> row 1, key 1 is invalid
values length 0, keys [0]    -> row 0, key 0 is invalid
```

## Convert Once to the Typed Family

The erased view identifies its physical type at runtime. The typed view chooses the scalar family
at compile time:

```rust,ignore
pub struct ColumnView<'a, S: Scalar> {
    kind: ColumnViewKind<'a, S>,
}

enum ColumnViewKind<'a, S: Scalar> {
    Array(&'a S::ArrayType),
    Constant {
        value: Option<S::RefType<'a>>,
        len: usize,
    },
    Dictionary {
        indices: &'a [Option<usize>],
        values: &'a S::ArrayType,
    },
}
```

Implement `TryFrom<ColumnViewImpl<'a>> for ColumnView<'a, S>`. First compare the view's runtime
physical type with `S::PHYSICAL_TYPE`. Then reuse the checked scalar and array conversions from
Chapter 1. Do not defer this check until `get`.

The typed `get(row)` returns `Option<S::RefType<'a>>`. It requires `row < len()` and follows strict
null propagation for the view itself:

- a null array slot returns `None`;
- a null constant returns `None` for every row;
- a null dictionary key returns `None`; and
- a non-null key pointing at a null dictionary value also returns `None`.

## Walk Through the Opening Example

Assume a future `contains` kernel receives the dictionary view and constant view above:

| Row | Key | Dictionary read | Constant read | Future `contains` result |
| ---: | ---: | --- | --- | --- |
| 0 | 2 | `"database"` | `"a"` | `true` |
| 1 | null | null | `"a"` | null |
| 2 | 1 | null | `"a"` | null |
| 3 | 0 | `"rust"` | `"a"` | `false` |

The future kernel is not part of this chapter. The useful result here is that it will not need to
know which encoding produced either borrowed input value.

## Behavioral Contract

Keep these invariants at the public constructor/conversion boundary:

1. `len()` is the array length, repeat count, or number of dictionary keys.
2. Every view has a `PhysicalType`, including empty views and null constants.
3. Null constants, null keys, and null dictionary values produce null logical rows.
4. `dictionary` is the only public dictionary construction path; it validates every non-null key
   and reports row, key, and values length.
5. Erased-to-typed conversion rejects a physical mismatch before `get` is called.
6. Views borrow their inputs and do not materialize or clone value buffers.
7. `get(row)` requires `row < len()`; an out-of-range row may panic.

## Implementation Checkpoints

Build the view in this order:

1. Add the opaque `ColumnViewImpl` and its constructors, `len`, `is_empty`, and `physical_type`.
2. Add complete dictionary-key validation and its comparable error.
3. Add the opaque typed `ColumnView<'a, S>` with `len`, `is_empty`, and `get`.
4. Implement the checked erased-to-typed conversion using Chapter 1 conversions.
5. Run the focused test and trace any failure through the seven invariants above.

## Review Your Chapter Result

Run the canonical command from the repository root:

```console
cargo test -p type-exercise-starter chapter_2 --locked
```

The expected result is four passing tests. They cover:

- regular arrays, non-null constants, and dictionaries through one typed interface;
- typed nulls, empty views, null keys, and null dictionary values;
- invalid keys against non-empty and empty dictionaries; and
- wrong physical array and null-constant types.

Then verify that Chapter 1 remains green:

```console
cargo test -p type-exercise-starter chapter_1 --locked
```

Before moving on, explain in your own words:

- what each view variant borrows;
- why `None` needs a separate physical type in a constant view;
- why dictionary construction validates keys before a typed read; and
- how one `ColumnView<'a, S>` preserves both the runtime representation choice and the compile-time
  scalar family.

Stop here. The next chapter will vectorize one scalar function over these views. It will decide how
to move representation dispatch out of the row loop; this chapter intentionally does not.

{{#include copyright.md}}
