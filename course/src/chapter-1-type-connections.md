# Chapter 1: Connect Scalars, References, and Arrays

Consider two nullable columns:

```text
integers: [Some(10), None, Some(30)]
strings:  [Some("db"), None, Some("rust")]
```

Reading the first column can copy an `i32`. Reading the second should return an `&str` that borrows
the array. You want one generic loop to handle both without converting every value into the runtime
`ScalarImpl` enum.

In this chapter, you will build the compile-time type family that makes that possible.

## Starting Point and Result

Start from `main`. The starter's implemented Rust model is only:

```rust
pub enum ScalarImpl {
    Int32(i32),
    String(String),
}
```

Use the course tool to copy the Chapter 1 contract into the starter, then run it once to see the
missing symbols:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter chapter_1 --locked
```

Before this chapter, the enum variants have no relationship to borrowed values or arrays. After
this chapter, the following two rows are enforced by associated types and checked conversions:

| Runtime type | Owned scalar | Borrowed scalar | Physical array |
| --- | --- | --- | --- |
| `Int32` | `i32` | `i32` | `I32Array` |
| `String` | `String` | `&'a str` | `StringArray` |

One generic function can then build, iterate, and inspect either nullable array. Converting a wrong
runtime variant returns `TypeMismatch` rather than panicking.

Plan on two study sessions, roughly four to eight hours if generic associated types and
higher-ranked trait bounds are new to you.

## Chapter Boundary

Modify only implementation files under `type-exercise-starter/src/`. You may add modules and
reorganize your implementation, but do not modify the copied test in
`type-exercise-starter/src/tests/chapter_1.rs` or the generated `src/tests.rs` module list.

Do not change the tests, dependencies, or `ScalarImpl` variants. Stop before macros,
additional data types, constants, dictionaries, expression traits, or generated code. Array buffer
layout is your choice as long as the observable contract holds and the code remains safe Rust.

## First Predict the Borrowed Types

Suppose `get` always returned an owned value. An integer read would be cheap, but a string read
would allocate and copy. Suppose it always returned a reference instead. Strings would be cheap,
but integers would become needlessly tied to an array borrow.

Predict the two return types before continuing:

```text
I32Array::get(row)    -> Option<?>
StringArray::get(row) -> Option<?>
```

The intended answers are `Option<i32>` and `Option<&str>`. The second type's lifetime must be tied
to the borrow of `StringArray`; the first type does not need to carry that lifetime. A generic
associated type lets one trait express both:

```rust,ignore
trait Array {
    type RefItem<'a>;

    fn get(&self, row: usize) -> Option<Self::RefItem<'_>>;
}
```

The `<'a>` is useful even when one implementation chooses a lifetime-independent type such as
`i32`.

## The Reciprocal Type Family

You will define four traits. Each associated type points to another member of the same row:

```text
S: Scalar                 ArrayType  -> A
                          RefType    -> R<'a>
R<'a>: ScalarRef<'a>      ScalarType -> S
                          ArrayType  -> A
A: Array                  OwnedItem  -> S
                          RefItem    -> R<'a>
                          Builder    -> B
B: ArrayBuilder           Array      -> A
```

The connections are reciprocal rather than one-way conveniences. For any supported scalar `S`
and its array `A`, all of these equations must hold:

1. `S::ArrayType = A`.
2. `S::RefType<'a> = A::RefItem<'a>`.
3. `<S::RefType<'a> as ScalarRef<'a>>::ScalarType = S`.
4. `<S::RefType<'a> as ScalarRef<'a>>::ArrayType = A`.
5. `A::OwnedItem = S`.
6. `A::Builder::Array = A`.

Some of those equalities must hold for every lifetime a caller might use. Rust writes that as a
higher-ranked trait bound:

```rust,ignore
trait Scalar: Sized
where
    for<'a> Self::ArrayType:
        Array<OwnedItem = Self, RefItem<'a> = Self::RefType<'a>>,
{
    type ArrayType;
    type RefType<'a>;
}
```

Read `for<'a>` as "for every possible borrow lifetime `'a`", not as one particular lifetime
chosen by the implementation. Use the same reading when you encode the arrows back from
`ScalarRef<'a>` and `Array`.

These are course rules. How you divide the modules and represent buffers are implementation
choices.

### `Scalar` and `ScalarRef`

`Scalar` represents an owned value. It supplies its runtime `PhysicalType`, array type, borrowed
type, and `as_scalar_ref` conversion. `ScalarRef<'a>` points back to the owned scalar and array and
can create an owned value with `to_owned_scalar`.

For `i32`, the owned and borrowed types are both `i32`; both conversions copy. For `String`,
`as_scalar_ref` calls `as_str`, while `&str::to_owned_scalar` allocates a new `String`.

Do not hide these two explicit implementations behind a macro. Seeing where their behavior differs
is the purpose of the exercise.

### `Array` and `ArrayBuilder`

`Array` supplies `Builder`, `OwnedItem`, and the lifetime-indexed `RefItem<'a>`. It also exposes
`get`, `len`, `is_empty`, `iter`, and `from_slice`. `ArrayBuilder` supplies `with_capacity`, `push`,
and `finish`.

For this chapter, `get(row)` requires `row < len()`; an out-of-range row may panic. A null row
returns `None`. `from_slice` and iteration must preserve both row count and null position.

A teaching implementation of `I32Array` may store `Vec<Option<i32>>`. A more Arrow-like
implementation may separate values and validity. `StringArray` may store owned strings or UTF-8
bytes plus offsets and validity. Whichever representation you choose, `get` must return an `&str`
that points into the array rather than a newly allocated string.

## Cross the Runtime Boundary Safely

Add the borrowed `ScalarRefImpl<'a>` and erased `ArrayImpl` enums with the same `Int32` and `String`
variants. Add `PhysicalType::{Int32, String}` and a comparable `TypeMismatch { expected, actual }`
error.

Implement the conversions in both directions manually:

- owned scalar to and from `ScalarImpl`;
- borrowed scalar to and from `ScalarRefImpl<'a>`;
- owned typed array to and from `ArrayImpl`; and
- borrowed `&ArrayImpl` to a borrowed typed array.

Upcasts with `From` cannot fail. Downcasts with `TryFrom` must report the expected and actual
physical types. A wrong variant is an ordinary boundary error, not an unreachable case.

Work through this mismatch before coding:

```text
requested type: &I32Array
runtime value:  ArrayImpl::String(...)
error:          expected Int32, actual String
```

The same rule applies to owned scalars and borrowed scalars.

## Implementation Checkpoints

Work in small slices so compiler errors stay local:

1. Add `PhysicalType`, `TypeMismatch`, and the borrowed scalar enum.
2. Define `Array` and `ArrayBuilder`; implement the integer and string arrays and one iterator.
3. Define `Scalar` and `ScalarRef<'a>`; connect both complete type-family rows explicitly.
4. Add the scalar and array `From`/`TryFrom` conversions.
5. Run the focused test and use its remaining failures as the next concrete task.

The compiler may show a long error when one reciprocal bound is missing. Read it from the first
unsatisfied associated-type equality: it usually identifies which arrow in the type-family diagram
has not yet been connected.

## Review Your Chapter Result

Run the canonical command from the repository root:

```console
cargo test -p type-exercise-starter chapter_1 --locked
```

The expected result is four passing tests. Together they cover:

- integer and string type-family associations;
- normal, null, and empty array reads;
- owned and borrowed erasure round trips; and
- wrong scalar and array variants.

Then run the baseline as a regression check:

```console
cargo test -p type-exercise-starter --lib --locked
```

Before moving on, explain in your own words:

- why `RefItem<'a>` is a generic associated type even though `i32` ignores `'a`;
- how the six reciprocal equations prevent an invalid scalar/array combination;
- which conversions borrow, copy, allocate, or move; and
- where a runtime mismatch is detected.

Stop after this explanation. [Chapter 2](./chapter-2-column-views.md) will use this type family to
read multiple column encodings; it will not add expressions yet.

{{#include copyright.md}}
