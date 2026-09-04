{{#include wip-banner.md}}

# Chapter 1: Build Physical Types and Arrays

Your first runnable product is a nullable column that stores several physical value families.
You will connect each owned scalar, borrowed scalar, array, and builder once, then erase those
concrete Rust types behind checked runtime enums.

Copy the cumulative Chapter 1 contract and see the unfinished boundary:

```console
cargo x copy-test --chapter 1
cargo test -p type-exercise-starter-supplied-tests chapter_1 --locked
```

The copied tests are course input. Implement the learner-owned files under
`type-exercise-starter/core/src/`; do not edit the copied test.

## Connect a family

Open `scalar.rs` and `array.rs`. Complete the reciprocal relationships represented by these
traits:

```text
owned scalar S <-> borrowed S::RefType<'a>
       |                    |
       +------ array -------+
                    |
                 builder
```

For integers, floats, and Boolean values, owned and borrowed values are cheap copies. A String is
owned as `String` but read as `&str`. Generic associated types let `String::RefType<'a>` preserve
the array's borrow instead of allocating per row.

Build fixed-width arrays as a values buffer plus a packed validity bitmap. A null still occupies
one ignored value slot; its validity bit is false. Build String arrays as UTF-8 bytes, monotonic
offsets, and the same packed validity shape. An empty string and a null can share offsets; only the
validity bit distinguishes them.

## Define the finite runtime catalog

Open `variant_catalog.rs`, `physical_type.rs`, and `scalar.rs`. Expand the catalog to Int16,
Int32, Int64, Bool, Float32, Float64, String, and Decimal. Use the catalog for the repetitive erased
variants and conversions while keeping the ordinary storage algorithms generic.

`From<T>` may erase a known concrete value without failure. `TryFrom<ArrayImpl>` and
`TryFrom<ScalarImpl>` must reject the wrong family with `TypeMismatch`; they must not panic or
reinterpret bytes.

Decimal is the intentional exception. Its precision and scale are runtime metadata shared by the
whole array, so validate them in `decimal.rs` and preserve them through scalar, array, and erased
boundaries. A failed decimal builder push must leave the builder unchanged.

Finally enable and export `data_type` and `decimal` in `core/src/lib.rs`. Logical types should map
to exactly one physical representation, while List stays commented for Chapter 9.

Run the complete chapter checkpoint:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_1 --locked
cargo test -p type-exercise-checkpoint-01-supplied-tests --locked
cargo check -p type-exercise-checkpoint-01-core --locked
```

The checkpoint is complete when all supported families preserve values and null positions,
checked erasure rejects cross-family recovery, String reads borrow their bytes, and Decimal retains
its exact metadata.

[Chapter 2](./chapter-2-column-views.md) will read these arrays without forcing every input into the
same storage shape.

{{#include copyright.md}}
