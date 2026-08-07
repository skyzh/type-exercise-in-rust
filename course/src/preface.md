# Build a Database Expression Framework in Rust

A database expression framework evaluates operations such as `integer + integer` or
`varchar contains varchar` over nullable batches. Its difficult boundary is not the scalar
function itself. It is keeping the relationships among owned values, borrowed values, physical
arrays, and runtime-erased inputs both correct and inexpensive.

You will build those relationships instead of receiving them pre-connected. The supplied starter
contains this enum:

```rust
pub enum ScalarImpl {
    Int32(i32),
    String(String),
}
```

The two variants are deliberately different. An integer can be copied out of an array. A string
should be returned as an `&str` that borrows the array's storage. By making one generic interface
work for both, you will encounter the reason this framework needs associated types, generic
associated types, lifetimes, and checked runtime erasure.

## Course Roadmap

The course has eight chapters. This is a dependency order, not an eight-day schedule:

| Chapter | Capability you add | Availability |
| --- | --- | --- |
| Connect Scalars, References, and Arrays | Build the reciprocal type families manually for integers and strings. | Available |
| Read Arrays, Constants, and Dictionaries | Expose three borrowed encodings as the same nullable logical rows. | Available |
| Vectorize a Scalar Function | Apply one typed scalar function to nullable column views. | Available |
| Erase and Generate Expressions | Support runtime arity and generate the repetitive typed adapters. | Available |
| Bind Logical Expressions | Reject invalid logical signatures and select a concrete kernel. | Planned |
| Specialize Primitive Loops | Add and measure all-valid primitive fast paths. | Planned |
| Strengthen Rust Type Boundaries | Exercise opaque iterators, variance, upcasting, and thread-safety contracts. | Planned |
| Add a Batch Async Boundary | Adapt whole batches without making each row evaluation asynchronous. | Planned |

You begin with two representative types and implement every connection explicitly. The first four
chapters keep type-family construction, column representation, typed evaluation, and runtime
erasure separate so that each chapter has one testable abstraction boundary. Macro expansion and
the broader type set come only after those boundaries are visible.

## What You Need to Know

You should be comfortable with Rust enums, structs, references, `Option`, traits, and associated
types. The first chapter introduces generic associated types in the concrete setting that needs
them. Familiarity with SQL nulls and columnar execution is helpful but not required.

Each implementation chapter ends with focused tests and a short explanation prompt. Passing the
tests is necessary, but you should also be able to explain the type or data-flow invariant and one
failure case in your own words.

Continue to [Environment Setup](./setup.md).

{{#include copyright.md}}
