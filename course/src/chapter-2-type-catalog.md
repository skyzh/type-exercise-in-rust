{{#include wip-banner.md}}

# Chapter 2: Scale the Physical Type Family

Chapter 1 connected Int32 and String by hand. An owned scalar, its borrowed form, its nullable
array, and its erased runtime variant now agree through Rust's associated types. That was useful
while there were two families. Adding five more primitive families by copying those
implementations would make the boilerplate larger than the storage idea.

This chapter separates ordinary generic storage from the finite runtime catalog. Six explicit
aliases name the supported primitive arrays, while one generic `PrimitiveArray<T>` implementation
owns their identical storage behavior. Its ordinary trait bounds require the complete scalar,
array, builder, and erased-conversion relationship. The physical
catalog remains for work Rust generics cannot express: erased enum variants and their
variant-specific conversions. Then we will add Decimal as the important exception: its precision
and scale are chosen at runtime. `DecimalArray` therefore wraps reused `PrimitiveArray<i128>`
coefficient and validity storage with one checked `DecimalType` shared by the whole array.

The goal is not to hide the type system behind a general framework. It is to keep two kinds of variation separate:

| | Static family identity | Runtime type metadata |
| --- | --- | --- |
| What varies? | The Rust scalar, borrowed scalar, array, and builder types | Values that describe one physical family at runtime |
| Examples | `i64` ↔ `I64Array`; `f64` ↔ `F64Array`; `String` ↔ `StringArray` | `DecimalType { precision, scale }` shared by a Decimal array |
| Where is it enforced? | Explicit public aliases plus the complete `Scalar`/`Array`/conversion bounds on the generic implementation | Checked constructors plus `PhysicalType::Decimal(decimal_type)` at erased boundaries |

## What is in the starter

Begin from your completed Chapter 1 workspace. `src/variant_catalog.rs` contains exactly the
Int32 and String rows. On Day 1, those rows drive only `PHYSICAL_FAMILY_CATALOG`; scalar and array
erasure remains handwritten in `src/scalar.rs` and `src/array.rs`. Day 2 first replaces that
Int32/String erasure with catalog callbacks, then extends the inventory. `src/physical_type.rs`,
`src/scalar.rs`, and `src/array.rs` expose those two physical families, and
`src/array/primitive_array.rs` has the working `I32Array` layout you implemented.

The Day 2 starter does not predeclare the rest of the solution. Comments in the active files mark
where the primitive variants and aliases belong. `src/data_type.rs`, `src/decimal.rs`, and
`src/array/decimal_array.rs` are still docstrings rather than executable declarations, and their
modules remain commented in `src/core.rs` and `src/array.rs`. You will make those files executable
only when their checkpoints introduce the concepts.

Copy the cumulative supplied test before editing:

```console
cargo x copy-test --chapter 2
cargo test -p type-exercise-starter-supplied-tests chapter_2 --locked
```

The Chapter 2 test is one final contract, not four progressive test files. Its first run should
fail because the new arrays, logical types, and Decimal types do not exist yet. Do not edit the
copied test. Checkpoints 1 and 2 can use a library check to catch local compiler errors. Checkpoints
3 and 4 share one compile boundary because the final `DataType` includes Decimal; enable their
modules together after both implementations exist. The focused Chapter 2 test becomes green after
all four checkpoints are complete.

```console
cargo check -p type-exercise-starter-expr --lib --locked
```

## Checkpoint 1: Generalize the primitive array family

Open `src/array/primitive_array.rs`. Chapter 1 implemented `Array` and `ArrayBuilder` directly for
the `I32Array` aliases. The storage does not depend on `i32`: every copyable primitive family uses
one contiguous `Vec<T>` plus one packed validity bitmap. What changes from family to family is the
Rust scalar type and the public alias name.

Generalize that implementation without inventing another marker trait. Write the six
public primitive aliases explicitly, for example:

```rust,ignore
pub type F64Array = PrimitiveArray<f64>;
pub type F64ArrayBuilder = PrimitiveArrayBuilder<f64>;
```

Implement `Array` once for `PrimitiveArray<T>` and `ArrayBuilder` once for
`PrimitiveArrayBuilder<T>`, with bounds connecting `T` to the matching `Scalar`, `ScalarRef`, and
erased `ArrayImpl` family. Those existing relationships are already the exact admission rule: an
arbitrary `PrimitiveArray<T>` remains useful as internal storage, but it does not become a
database `Array` unless `T` satisfies the complete static family contract. Do not duplicate that
contract with a private marker trait.

Keep the Chapter 1 layout unchanged. `push(None)` still appends a default placeholder and a false
validity bit. `get` consults validity before returning the copied value. NaN, infinity, and signed
zero are stored as their original floating-point bit patterns; do not add an equality or ordering
requirement just to make the generic implementation convenient.

The six aliases are ordinary Rust declarations, not generated execution code. Re-export them from
`src/array.rs`; their scalar and erased-enum relationships become complete when Checkpoint 2 adds
the remaining physical catalog rows.

```console
cargo check -p type-exercise-starter-expr --lib --locked
```

## Checkpoint 2: Make the catalog own the repeated relationships

Now open `src/variant_catalog.rs`. Each row names six facts that otherwise have to stay synchronized:

```text
storage kind, erased variant, array, builder, owned scalar, borrowed scalar
```

Extend the inventory with the static Day 2 families: Int16, Int64, Bool, Float32, and Float64.
Int32 remains in place, and String remains the one `borrowed` row because its array yields `&str`
rather than copying an owned `String`.

Add catalog callbacks in `src/scalar.rs` and `src/array.rs` that replace the handwritten
Int32/String erasure. Then use those callbacks to generate the new erased scalar and array
variants, scalar-family relationships, physical-type dispatch, and variant-specific checked
conversions without five hand-written copies. They do not generate the primitive aliases or
duplicate the generic `Array` implementation from Checkpoint 1. Add the
matching variants to `PhysicalType` and `PhysicalFamily` in `src/physical_type.rs`, and keep
`PHYSICAL_FAMILY_CATALOG` in the same public order. The supplied test treats that public list as an
audit surface: an omitted, duplicated, or misnamed family is a failure even if some generated code
still compiles.

This is a declarative macro, not runtime reflection. After expansion, Rust still sees concrete
items such as `impl Scalar for f64`, `ArrayImpl::Float64(F64Array)`, and a checked
`TryFrom<ArrayImpl> for F64Array`. The compiler checks the same reciprocal relationships from
Chapter 1 for every static row:

```text
owned scalar <-> borrowed scalar <-> concrete array <-> builder
```

Do not implement the static `Scalar`/`Array` family contract for `i128`. Checkpoint 4 will add
Decimal's catalog row after the descriptor-bearing types exist. The Decimal wrapper may reuse `PrimitiveArray<i128>` as internal
coefficient and validity storage, but its builder still needs a runtime `DecimalType` before it can
accept any row.

```console
cargo check -p type-exercise-starter-expr --lib --locked
```

## Checkpoint 3: Separate logical type from physical storage

So far, `PhysicalType` answers an execution question: which scalar and array representation is in
memory? A planner asks a different question. SQL `CHAR(7)` and `VARCHAR` have different logical
meaning, but this course stores both in the String physical family. That distinction belongs in a
planner-visible `DataType`, not in `StringArray`.

Replace the docstring in `src/data_type.rs` with `DataType` and its methods. Add the primitive
logical variants and the two string variants, then map them explicitly:

| Logical `DataType` | Physical storage |
| --- | --- |
| `SmallInt` | `Int16` |
| `Integer` | `Int32` |
| `BigInt` | `Int64` |
| `Boolean` | `Bool` |
| `Real` | `Float32` |
| `Double` | `Float64` |
| `Varchar`, `Char { width }` | `String` |

Implement `physical_type`, `is_string`, and `is_numeric`. `Boolean` is not numeric. The `width` in
`Char { width }` remains logical metadata even though it does not change the physical array.

Do not add a nullable logical variant or List. Keep one primitive array representation with its
values and packed validity bitmap. Day 7 will borrow those two buffers through a crate-private raw
view, run strict total `i32` operations over values, and combine validity separately by storage
word. Constants use the same raw route through one copied value plus a validity bit; Indexed views
keep the general gather loop. This requires neither a second Arrow array type nor a public
all-valid proof. List arrives with its own scalar and array relationships on Day 12.

Checkpoint 4 adds the Decimal variants and checked constructor to this same file. After that work,
uncomment the `data_type` and `decimal` modules and exports in `src/core.rs`; do not enable any
later-day module.

## Checkpoint 4: Keep Decimal metadata with the physical value

An `f64` value carries its interpretation in its bits. An `i128` coefficient does not tell you
whether `12345` means `12345`, `123.45`, or `12.345`. Decimal therefore cannot use the static
primitive relationship unchanged.

The starter already includes `anyhow`; do not change its manifest or the workspace lockfile here.
Implement `DecimalType` and `Decimal` in
`src/decimal.rs` with `anyhow::Result`. This chapter needs readable checked failures, not a public
Decimal-specific error taxonomy. `DecimalType` owns the
precision and scale and accepts only:

```text
1 <= precision <= 38
0 <= scale <= precision
```

The scale is an unsigned `u8`, so it is nonnegative by construction. A `Decimal` pairs one checked
`i128` coefficient with a `DecimalType`; its represented value is
`unscaled * 10^(-scale)`. A coefficient is valid when its absolute value is strictly less than
`10^precision`. Use an overflow-safe absolute value so `i128::MIN` returns an ordinary error
instead of panicking.

Next implement `DecimalArray` and `DecimalArrayBuilder` in `src/array/decimal_array.rs`. `DecimalArray` is a logical metadata wrapper around `PrimitiveArray<i128>`:

| Stored state | Role |
| --- | --- |
| `DecimalType { precision, scale }` | One checked descriptor shared by the entire array |
| `PrimitiveArray<i128>` | One flat coefficient slot and one validity bit per row |

Do not cache a null count. The reused primitive representation already owns the coefficient buffer and validity bitmap.

Require the descriptor before the first push with `DecimalArrayBuilder::try_with_type`. Store zero as the ignored coefficient for a null row, just as other primitive arrays use a placeholder value. Empty and all-null arrays must retain their `DecimalType`; the descriptor cannot be inferred from a non-null row because such a row may not exist.

Validate before mutation. `try_from_raw_parts` rejects different value/validity lengths and any valid coefficient outside the declared precision. `try_push` rejects a `Decimal` whose descriptor does not exactly match the builder's descriptor, without appending either a coefficient or a validity bit. A failed push must leave the builder in the same logical state it had before the call.

Now complete the Decimal path through the runtime types:

- add `DataType::Decimal(DecimalType)` and the checked `DataType::decimal` constructor;
- add `PhysicalType::Decimal(DecimalType)` and the descriptor-free `PhysicalFamily::Decimal` audit
  tag;
- add the `decimal` row to `for_each_physical_family!` and to `PHYSICAL_FAMILY_CATALOG`;
- enable and re-export `decimal_array` from `src/array.rs`; and
- make `ScalarImpl`, `ScalarRefImpl`, and `ArrayImpl` report the exact descriptor from `physical_type()`.

Do not add a `try_decimal(expected)` convenience method. A caller that requires one precision and scale first compares the erased value's `physical_type()` with `PhysicalType::Decimal(expected)`. Only after equality does it use the existing checked conversion—`Decimal::try_from`, `<&DecimalArray>::try_from`, or owned `DecimalArray::try_from`—when it actually needs a typed value. A descriptor mismatch is a physical-type mismatch at the caller; converting the wrong erased family returns an ordinary `anyhow` failure such as `expected a Decimal value, got Int32`. Code that only carries an erased value forward does not need to force a Decimal conversion.

Decimal does not implement the Chapter 1 `Scalar`/`Array` static-family contract because that contract fixes the physical type in the Rust type relationship and constructs builders with `ArrayBuilder::with_capacity(capacity)`. Decimal's precision and scale are runtime values, and an empty or all-null builder cannot infer them from a row. `DecimalArrayBuilder::try_with_type(decimal_type, capacity)` must therefore receive the descriptor up front. The Decimal catalog arm generates only the erased enum plumbing for this metadata-bearing wrapper; it neither duplicates primitive storage nor repeats precision and scale in every row.

This chapter does not implement Decimal arithmetic, comparison, rounding, casts, or implicit
coercion. It establishes the representation and checked runtime boundary that those operations
would have to preserve.

Run the final contract and the starter library tests:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_2 --locked
cargo test -p type-exercise-starter-expr --lib --locked
```

Before continuing, make sure you can explain three boundaries in your own words:

1. Which existing scalar, array, and conversion bounds admit `PrimitiveArray<f64>` to the generic
   database-array implementation, while an arbitrary `PrimitiveArray<T>` does not qualify?
2. Why do `Char { width }` and `Varchar` remain distinct logical types even though both map to
   `PhysicalType::String`?
3. Why can `DecimalArray` reuse `PrimitiveArray<i128>` storage while `DecimalArrayBuilder` still cannot use the metadata-free `ArrayBuilder::with_capacity` constructor?

You now have one compile-time inventory for the repeated static relationships and one explicit,
checked path for runtime metadata. Chapter 3 will use those physical families through several
nullable column encodings.

Next: [Chapter 3 reads several nullable column encodings](./chapter-3-column-views.md).

{{#include copyright.md}}
