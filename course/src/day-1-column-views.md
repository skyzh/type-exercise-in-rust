# Day 1: Read Arrays, Constants, and Dictionaries

Suppose a string function receives four logical rows:

```text
dictionary values: ["rust", "database", "type system"]
row keys:          [0,      1,          null, 2]
constant needle:   "a"
```

The function should observe `["rust", "database", null, "type system"]` on the left and four copies
of `"a"` on the right. It should not expand either input into a temporary `StringArray`.

Today you will build a borrowed column view that exposes those logical rows while preserving the
existing typed kernel interface.

## Starting Point and Result

The starter already connects three representations of a value:

```text
owned scalar        borrowed scalar        physical array
String              &'a str                StringArray
i32                 i32                    PrimitiveArray<i32>
List                ListRef<'a>            ListArray
```

`Scalar` and `ScalarRef` express those relationships with generic associated types. `ArrayImpl` and
`ScalarRefImpl<'a>` erase the concrete type at runtime boundaries.

Before this day, `Expression::eval_expr` accepts only regular arrays. After this day:

- `Expression::eval` accepts borrowed `ColumnViewImpl<'a>` inputs;
- `ColumnView<'a, S>` gives a typed kernel one interface for arrays, constants, and dictionaries;
- invalid physical types and dictionary keys fail before the row loop; and
- the array-only API remains as a compatibility adapter.

This day does not add planning-time logical type checks, primitive-only fast paths, or asynchronous
evaluation. Those belong to later days.

## The Column Contract

Implement the contract in `expr-common/src/column.rs` and adapt the expression interface in
`expr-common/src/expr.rs`.

1. `len()` is the number of logical rows: array length, constant repeat count, or dictionary key
   count.
2. A null constant still carries a `PhysicalType`; `None` alone cannot distinguish a null integer
   from a null string.
3. A null dictionary key produces a null row. A non-null key selects one value from the dictionary
   array.
4. `ColumnViewImpl::dictionary` rejects every key outside `0..values.len()` and reports both the
   logical row and invalid key.
5. Converting `ColumnViewImpl<'a>` to `ColumnView<'a, S>` checks the physical type before the typed
   row loop begins.
6. Views borrow existing buffers. Creating or copying a view must not clone or materialize values.
7. All inputs to one expression evaluation have equal lengths. The generated evaluator checks this
   once before processing rows.

The representation of dictionary keys and the choice to validate all keys at construction are
course rules. Naming local helpers and arranging the three typed accessor structs are ordinary
implementation choices.

## Typed Values Before Type Erasure

An array stores nullable values of one physical type. Fixed-width arrays keep a value buffer and a
validity bitmap. `StringArray` keeps UTF-8 bytes, offsets, and a validity bitmap:

```text
logical: [Some("db"), None, Some("rust")]
data:    dbrust
offsets: 0, 2, 2, 6
valid:   1, 0, 1
```

The borrowed item depends on the array borrow: an integer is copied, while a string is returned as
`&'a str`. The starter's `Scalar` relation lets a generic column view recover the right array and
borrowed item from `S`:

```rust,ignore
pub trait Scalar {
    const PHYSICAL_TYPE: PhysicalType;
    type ArrayType: Array<OwnedItem = Self>;
    type RefType<'a>: ScalarRef<
        'a,
        ScalarType = Self,
        ArrayType = Self::ArrayType,
    >;
}
```

That is why the typed view uses `ColumnView<'a, S>` instead of independently choosing a scalar,
borrowed scalar, and array type.

## Three Representations, One Read

The erased boundary stores only borrowed inputs:

```rust,ignore
pub enum ColumnViewImpl<'a> {
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

After a checked conversion, the typed view can return `Option<S::RefType<'a>>` for every
representation. Match the representation outside the hot loop when possible; the loop should call
a statically typed accessor rather than downcast each row.

For the opening example, work through the rows by hand:

| Row | Key | Dictionary result | Constant | `contains` result |
| ---: | ---: | --- | --- | --- |
| 0 | 0 | `"rust"` | `"a"` | `false` |
| 1 | 1 | `"database"` | `"a"` | `true` |
| 2 | null | null | `"a"` | null |
| 3 | 2 | `"type system"` | `"a"` | `false` |

The null result follows from strict SQL-style evaluation: if either input is null, the kernel is
not called for that row.

## Implementation Checkpoints

Work in this order:

1. Add `ColumnViewImpl` constructors, `len`, `physical_type`, and dictionary-key validation.
2. Add typed array, constant, and dictionary accessors implementing `ColumnAccessor<'a, S>`.
3. Implement the checked conversion to `ColumnView<'a, S>`.
4. Change `Expression::eval` to accept column views and retain `eval_expr` as the array adapter.
5. Update the generated expression template to dispatch representations once per batch and verify
   equal input lengths.

Keep changes inside `expr-common`, `expr-template`, and `expr-template-impl`. Do not add the
planning registry or a primitive specialization yet.

## Verify the Day

Run the focused tests:

```console
cargo test -p expr-common column --locked
cargo test -p expr-impl --locked
```

The first command should pass the normal array/constant/dictionary case and the invalid-key case.
The second should preserve the existing array-only expression behavior through the compatibility
adapter.

Before moving on, explain:

- why a typed null needs `PhysicalType`;
- where dictionary keys are validated and why the kernel may then index safely;
- which data the three view variants borrow; and
- why matching a representation once per batch is different from materializing it.

Next, you will bind logical function signatures before evaluation.

{{#include copyright.md}}
