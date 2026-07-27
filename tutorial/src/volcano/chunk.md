# Values, Chunks, and Nulls

A SQL type is not the same thing as a Rust storage type. `CHAR(10)` and `VARCHAR` are distinct
logical types, but both can use the same offsets-plus-bytes string array. `DECIMAL(10, 2)` and
`DECIMAL(30, 8)` can share a physical decimal representation while carrying different rules during
planning.

The repository records logical types in `DataType`:

```rust
pub enum DataType {
    SmallInt,
    Integer,
    BigInt,
    Varchar,
    Char { width: u16 },
    Boolean,
    Real,
    Double,
    Decimal { scale: u16, precision: u16 },
}
```

`DataType::physical_type` deliberately collapses logical distinctions:

```rust,ignore
DataType::Varchar          -> PhysicalType::String
DataType::Char { .. }      -> PhysicalType::String
DataType::Decimal { .. }   -> PhysicalType::Decimal
```

The binder reasons about `DataType`; the vectorized runtime dispatches `PhysicalType`.

## A Row and a Chunk

Conceptually, a row is a sequence of nullable scalar values:

```text
[Some(Int32(42)), Some(String("db")), None]
```

A columnar chunk transposes many rows:

```text
Int32 column:  [42, 7, 9]
String column: ["db", "rust", "sql"]
Boolean column:[true, NULL, false]
```

The transpose matters for performance, not semantics. The result at logical row `r` must be the
same whether an expression reads a row object or reads position `r` from each column.

## Null Is Part of the Value Contract

For this course, ordinary scalar functions are *strict*: if any input at a row is null, the output
is null. The vectorizer implements this rule around a non-null scalar function:

```rust
fn str_contains(left: &str, right: &str) -> bool {
    left.contains(right)
}
```

The function does not accept `Option<&str>`. The common loop handles the nullable cases. This keeps
simple kernels simple, but it is not universal SQL behavior. `IS NULL`, `COALESCE`, and three-valued
boolean logic need custom null policies. In a production registry, null behavior is function
metadata; in this course, those functions would use a custom expression implementation.

That distinction is another example of the main rule: abstract a behavior only when many functions
actually share it.

## Physical Layout Is Not Logical Shape

Even “one column” may have several physical encodings:

- a regular values buffer plus null bitmap;
- a dictionary-values array plus row indices;
- a constant value repeated for every row; or
- an imported Arrow array.

All expose the same logical sequence of `Option<ScalarRef>`. Part II will capture that observation
in `ColumnView` instead of materializing each encoding into a regular array.

## Chapter Checkpoint

You should now be able to separate three concepts:

1. `DataType`: the SQL-level contract used by the binder;
2. `PhysicalType`: the runtime representation used for dispatch; and
3. nullable scalar values at a particular logical row.

Next, use those values to [evaluate scalar expressions](./expressions.md).

## Test Your Understanding

- Why should `CHAR(10)` and `VARCHAR` remain different during binding even if both use `StringArray`?
- Where should strict null propagation live: in every scalar function or in the vectorizer?
- Name an expression that cannot use strict null propagation.

{{#include ../copyright.md}}
