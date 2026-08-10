# Chapter 1: Connect One Type Family by Hand

An `i32` can be copied out of an array. A `String` should be read as an `&str` that borrows the
array. This chapter connects both cases without forcing one into the other's ownership model.

**Prerequisites:** enums, traits, references, associated types, and `Option`.

**By the end of this chapter, you will:**

- connect owned values, borrowed values, arrays, and builders for `i32` and `String`;
- use a generic associated type for the borrowed member of each family; and
- upcast typed values with `From` and downcast erased enums with checked `TryFrom` conversions.

## See the missing connections

Copy the cumulative contract and run it once:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter chapter_1 --locked
```

The untouched starter compiles. Follow the `Day 1, checkpoint ...` comments beside the declarations
in the named starter files; the focused test should then fail at the corresponding `todo!` boundary
until you implement it. Do not edit the copied test.

The family you are building has reciprocal arrows:

```text
Scalar ──RefType<'a>──> ScalarRef<'a>
  │                         │
ArrayType                ArrayType
  ▼                         ▼
Array ─────Builder─────> ArrayBuilder
```

For `i32`, the borrowed scalar is another `i32`. For `String`, it is `&'a str`. `Array::get`
therefore returns `Option<i32>` for `I32Array` and `Option<&str>` for `StringArray` without an
allocation on the string read.

## Checkpoint 1: describe the two physical families

- **Target:** `type-exercise-starter/src/physical_type.rs::{PhysicalType, TypeMismatch}` and
  `type-exercise-starter/src/scalar.rs::{Scalar, ScalarRef, ScalarImpl, ScalarRefImpl}`.
- **Change:** add only the `Int32` and `String` physical rows and their reciprocal associated
  types.
- **Preserve:** the original two `ScalarImpl` variants and safe Rust.
- **Run:** the Chapter 1 focused test.
- **Passing means:** owned and borrowed values point to the correct array family.

`for<'a>` on a bound means the relationship holds for every caller-chosen borrow lifetime. The
integer implementation may ignore that lifetime; the string implementation cannot.

## Checkpoint 2: store nullable rows

- **Target:** `type-exercise-starter/src/array.rs::{Array, ArrayBuilder, ArrayImpl}`,
  `type-exercise-starter/src/array/primitive_array.rs::{PrimitiveArray, PrimitiveArrayBuilder}`, and
  `type-exercise-starter/src/array/string_array.rs::{StringArray, StringArrayBuilder}`.
- **Change:** implement `get`, `len`, `iter`, `from_slice`, builder `push`, and `finish` for the two
  families. Expose read-only `values`/`validity` accessors on `PrimitiveArray` and
  `data`/`offsets`/`validity` accessors on `StringArray` so the buffer contract is visible.
- **Preserve:** row count and null positions; returned strings must borrow array storage; offsets
  count UTF-8 bytes rather than characters.
- **Run:** the same focused test.
- **Passing means:** normal, null, and empty arrays read through one generic contract.

Use the Arrow-like layout required by the supplied test. A fixed-width array stores one flat
`Vec<T>` with exactly one slot per row. A string array stores all UTF-8 bytes in one flat `Vec<u8>`
and uses `rows + 1` monotone offsets: the first is zero, the last is the byte-buffer length, and a
null or empty row repeats an offset. Both arrays store row validity in the packed
`bitvec::vec::BitVec` already declared in the starter. A null row returns `None`, but it is never an
`Option<T>` payload slot; strings are not stored as one owned `String` per row.

## Checkpoint 3: erase and recover values

- **Target:** `From`/`TryFrom` implementations in `type-exercise-starter/src/scalar.rs` and
  `type-exercise-starter/src/array.rs`, plus exports in `type-exercise-starter/src/lib.rs`.
- **Change:** upcast typed values into erased enums and recover the requested type.
- **Preserve:** wrong variants return `Err`; they do not panic. The starter's readable
  `TypeMismatch` is available, but copied tests do not require a particular field layout or message.
- **Run:** the focused and cumulative starter tests.
- **Passing means:** correct variants round-trip and wrong variants fail at the boundary.

## Required and extension work

Required work is exactly the explicit `i32` and `String` rows. Additional physical types, macros,
columns, and expressions belong to later chapters. As an extension, sketch a third family on paper
and mark every enum arm and conversion it would require; do not implement it yet.

```console
cargo test -p type-exercise-starter chapter_1 --locked
cargo test -p type-exercise-starter --lib --locked
```

Before continuing, explain why `StringArray::get` needs a lifetime-indexed associated type and why
an erased downcast is fallible even when the compile-time family is consistent.

Next: [Chapter 2 scales the family without copying every connection](./chapter-2-type-catalog.md).

{{#include copyright.md}}
