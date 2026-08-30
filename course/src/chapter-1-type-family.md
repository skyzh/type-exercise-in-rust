{{#include wip-banner.md}}

# Chapter 1: Connect One Type Family by Hand

In this chapter, you will use generic associated types (GATs) to connect the different representations of one database value: an owned scalar, a borrowed scalar reference, and a nullable array. We will make those connections for `i32` and `String`. The same relationships will later let us implement primitive arrays once and write generic expression code without repeating it for every physical type.

A database execution engine rarely works with one Rust representation of a value. A string may arrive as an owned `String`, be read from an array as an `&str`, and live inside a compact column with thousands of other strings. These are different Rust types, but the engine must know that they belong to the same logical family.

The relationships we will build look like this:

```text
owned scalar S  ── RefType<'a> ──> borrowed scalar S::RefType<'a>
      │                                      │
      └──────────── ArrayType ───────────────┘
                             │
                             ▼
                       concrete array
```

For the integer family, the owned and borrowed representations are both `i32` because copying an integer is cheap. For the string family, the owned representation is `String`, while the borrowed representation is `&'a str`. The lifetime `'a` ties the borrowed string to the scalar or array that stores its bytes.

## What is in the starter

The Day 1 starter is deliberately small. It exposes only the two families used in this chapter: Int32 and String. `PhysicalType` and `PhysicalFamily` contain those two variants; `ScalarImpl`, `ScalarRefImpl`, and `ArrayImpl` contain their two erased variants. The starter also provides placeholder `PrimitiveArray<T>` and `StringArray` types and their builders. These declarations let the crate compile, but they do not implement the family relationships or store any values yet.

The `Scalar`, `ScalarRef`, `Array`, and `ArrayBuilder` traits begin as unbounded shells. They name the associated types and operations you will connect, but their supertraits, `where` clauses, and reciprocal associated-type bounds are learner work. Later physical types and later-day relationships appear only in comments or docstrings; they are not executable scaffolding for you to work around.

The comments beside each missing relationship name the checkpoint that owns it. Treat those comments as the implementation boundary: complete the two Day 1 families, but do not add the later families yet.

Copy the supplied Chapter 1 test and run it once before editing the starter:

```console
cargo x copy-test --chapter 1 --checkpoint 1
cargo test -p type-exercise-starter-expr chapter_1 --locked
```

The test is cumulative course material; do not edit the copied file. Work only in the learner files named below.

## Checkpoint 1: Implement the `Scalar` and `ScalarRef` traits

Open `src/scalar.rs`. The starter already distinguishes owned Int32 and String values in `ScalarImpl` and borrowed Int32 and String values in `ScalarRefImpl<'a>`. What it does not yet express is that each owned type has exactly one borrowed type and one array type, and that the borrowed type points back to the same family.

Complete only the owned↔borrowed bounds and associated-type relationship on `Scalar` and `ScalarRef`. `Scalar::ArrayType` and `ScalarRef::ArrayType` remain unconstrained associated-type placeholders in this checkpoint; do not require them to implement `Array` or tie them reciprocally yet. Use `RefType<'a>` and `ScalarType` to make the owned and borrowed directions agree: if `String::RefType<'a>` is `&'a str`, then that reference must identify `String` as its owned scalar. Checkpoint 3 will connect both scalar forms to the concrete array and builder once those implementations exist.

Then implement the owned↔borrowed relationship for the two families. The array names remain placeholders until Checkpoint 3:

```text
i32    <──owned/borrowed──> i32       <──array──> I32Array
String <──owned/borrowed──> &'a str   <──array──> StringArray
```

This is where the GAT matters. A normal associated type could say that `String` has some reference type, but it could not produce a different `&'a str` for every lifetime chosen by the caller. `type RefType<'a>` preserves that caller-chosen lifetime.

Why spend this effort on relationships before implementing an expression engine? Consider nullable equality, a common database scalar operation:

```rust,ignore
use crate::Scalar;

fn nullable_eq<'a, S>(
    left: Option<S::RefType<'a>>,
    right: Option<S::RefType<'a>>,
) -> Option<bool>
where
    S: Scalar,
    S::RefType<'a>: PartialEq,
{
    match (left, right) {
        (Some(left), Some(right)) => Some(left == right),
        _ => None,
    }
}

assert_eq!(nullable_eq::<i32>(Some(7), Some(7)), Some(true));
assert_eq!(nullable_eq::<String>(Some("db"), Some("rust")), Some(false));
assert_eq!(nullable_eq::<String>(None, Some("rust")), None);
```

The function describes the database rule once: compare two non-null values, otherwise produce `NULL`. The type family decides whether the compared values are copied integers or borrowed strings:

Chapter 1 does not build the generic expression framework yet. The supplied Checkpoint 1 compile witness uses this same idea to prove that each owned scalar and borrowed scalar point back to one another. When this checkpoint passes, generic code can name `S::RefType<'a>` without separately teaching it the Int32 and String cases; no `Array` or builder relationship is required yet.

Run the focused test again:

```console
cargo x copy-test --chapter 1 --checkpoint 1
cargo test -p type-exercise-starter-expr chapter_1 --locked
```

## Checkpoint 2: Add scalar type erasure

The traits from Checkpoint 1 work when Rust knows the concrete type `S` at compile time. A database plan does not always have that information in its Rust type. A scan may read a runtime schema, or an expression node may hold a value whose physical type is known only after binding. We therefore need one runtime container for all scalar families supported so far.

That is the role of `ScalarImpl` and `ScalarRefImpl<'a>` in `src/scalar.rs`. The first owns a value; the second can borrow one. For Day 1, each enum contains only Int32 and String. The enums erase the concrete Rust type at the runtime boundary while their variants preserve enough information to recover it safely.

Implement the Day 1 erased methods and conversions in `src/scalar.rs` and the display/error behavior of the existing `TypeMismatch` carrier in `src/physical_type.rs`:

- `ScalarImpl::physical_type` and `ScalarRefImpl::physical_type` report the variant's `PhysicalType`.
- `ScalarRefImpl::to_owned_scalar` turns an erased borrowed value into the matching erased owned value.
- `From<T>` moves a correctly typed value into its erased enum and cannot fail.
- `TryFrom<ScalarImpl>` and `TryFrom<ScalarRefImpl<'a>>` recover a requested concrete type.
- A matching variant returns the value.
- A nonmatching variant returns `TypeMismatch`; it must not panic or reinterpret the value.

For example, converting `42_i32` into `ScalarImpl` and back to `i32` succeeds. Asking for a `String` from that same `ScalarImpl::Int32` returns an error. This fallibility is why the generic traits alone are not enough: generics prevent a mismatch inside statically typed code, while an erased runtime boundary must check the variant it receives.

Keep the distinction between owned and borrowed erasure visible. `ScalarImpl::String` owns a `String`; `ScalarRefImpl::String` holds an `&str` with the caller's lifetime. Do not allocate a new `String` merely to erase a borrowed value.

Run the same focused test. Its scalar-erasure checkpoint should now round-trip both families and reject a cross-family downcast.

```console
cargo x copy-test --chapter 1 --checkpoint 2
cargo test -p type-exercise-starter-expr chapter_1 --locked
```

## Checkpoint 3: Implement primitive and string arrays

Now connect the array-type placeholders from Checkpoint 1 to concrete arrays. Open `src/array.rs`, `src/array/primitive_array.rs`, and `src/array/string_array.rs`. Add the reciprocal Scalar↔Array and Array↔ArrayBuilder bounds here, then implement the storage and access methods for `I32Array` and `StringArray`.

Why arrays? Database execution engines usually process columns in batches instead of dispatching one operator for every row. Conceptually, a vectorized binary expression performs the same scalar operation across two input arrays:

```rust,ignore
for row in 0..input_len {
    result.push(scalar_func(left[row], right[row]));
}
return result;
```

Later chapters will build the reusable vectorization layer. In this chapter, the goal is the representation underneath it: the array must return the scalar reference associated with its family, preserve nulls, and support append-only construction.

Use the small Arrow-style layouts required by the supplied tests. They are teaching layouts inspired by Arrow's columnar separation; this chapter does not claim full Apache Arrow compatibility.

For `PrimitiveArray<i32>`, store:

- one contiguous `Vec<i32>` with one value slot per row; and
- one packed `BitVec` validity bitmap, where `true` means the row is non-null.

A null integer row still occupies a value slot, using the type's default value as an ignored placeholder. Nullness comes from the validity bit, not from wrapping every stored value in `Option<i32>`.

For `StringArray`, store:

- one contiguous `Vec<u8>` containing the UTF-8 bytes for all rows;
- an offsets vector with `row_count + 1` entries; and
- one packed `BitVec` validity bitmap.

Row `i` occupies the half-open byte range `offsets[i]..offsets[i + 1]`. The first offset is zero, the offsets never decrease, and the last offset is the byte-buffer length. A null row and an empty string may repeat an offset; the validity bit distinguishes them. Because the bytes live in the array, `StringArray::get` returns an `&str` borrowed from that buffer rather than allocating a `String`.

Implement the Day 1 array surface described by the starter comments:

- array access: `get`, `len`, `is_empty`, `iter`, and the read-only buffer accessors used by the tests;
- construction: `with_capacity`, `push`, and `finish` on each builder; and
- `Array::from_slice`, which builds an array through its associated builder.

Preserve the row count and null position for normal, empty, and all-null inputs. String offsets count UTF-8 bytes, not characters.

```console
cargo x copy-test --chapter 1 --checkpoint 3
cargo test -p type-exercise-starter-expr chapter_1 --locked
```

When this checkpoint passes, the scalar relationship from Checkpoint 1 becomes observable: `I32Array::get` produces `Option<i32>`, while `StringArray::get` produces `Option<&str>` borrowing the array.

## Checkpoint 4: Add array type erasure with a macro

Concrete arrays are ideal for generic code, but a database operator often receives a column selected from a runtime schema. `ArrayImpl` in `src/array.rs` is the erased boundary for that case. On Day 1 it has only `Int32(I32Array)` and `String(StringArray)` variants.

Implement the common erased-array operations and the checked conversions between each concrete array and `ArrayImpl`. As with scalar erasure, upcasting with `From` cannot fail, while downcasting with `TryFrom` must return `TypeMismatch` for the wrong variant. Support both owned recovery and borrowed recovery so callers can inspect an erased array without cloning its buffers.

The two families need the same conversion shape. Write that shape once as a `macro_rules!` macro, then invoke it for Int32 and String. Keep the family inventory in `src/variant_catalog.rs` to exactly the two Day 1 rows. The catalog supplies the type names to the macro; it must not contain the later physical families yet.

The point of this macro is narrow: remove repetitive enum conversion code while keeping each generated implementation ordinary, inspectable Rust. It is not a generic reflection system. Later chapters will extend the catalog and reuse the same expansion boundary.

Finish by checking these behaviors:

- an `I32Array` and a `StringArray` each round-trip through `ArrayImpl`;
- borrowed recovery returns a reference to the original concrete array;
- asking for `I32Array` from `ArrayImpl::String` returns `TypeMismatch`;
- erased `get` preserves nulls and returns the matching `ScalarRefImpl` variant; and
- the physical-family catalog contains exactly Int32 and String.

Run the focused test, then the starter library tests:

```console
cargo x copy-test --chapter 1 --checkpoint 4
cargo test -p type-exercise-starter-expr chapter_1 --locked
cargo test -p type-exercise-starter-expr --lib --locked
```

Before continuing, make sure you can explain three boundaries in your own words:

1. Why does `String` need `RefType<'a> = &'a str`, while `i32` can use `RefType<'a> = i32`?
2. Why can generic code trust a `Scalar` relationship, while erased code must perform a checked downcast?
3. Which bytes represent a null string row, and which structure tells you that it is null rather than empty?

You have connected the first two concrete families by hand. Chapter 2 will extend the physical-family catalog and let the macros reproduce those connections for more types without turning the Day 1 starter into a completed framework.

Next: [Chapter 2 scales the family without copying every connection](./chapter-2-type-catalog.md).

{{#include copyright.md}}
