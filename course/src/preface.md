# Build a Typed Database Expression Engine in Rust

A hand-written loop for `i32 + i32` is easy:

```rust,ignore
for row in 0..left.len() {
    output.push(match (left.get(row), right.get(row)) {
        (Some(left), Some(right)) => Some(
            std::ops::Add::add(std::num::Wrapping(left), std::num::Wrapping(right)).0,
        ),
        _ => None,
    });
}
```

The design problem appears when the engine must also borrow strings without copying, read
constants and Indexed views, promote mixed numeric types, reject bad arity and lengths, and choose
a function from runtime names. Repeating those decisions in every loop makes each new function a
new place for type drift, null bugs, and inconsistent errors.

This course builds the connections that move those decisions out of the row loop. The workspace
starts with two crates: `type-exercise-starter-core` owns storage, views, and reusable evaluators;
`type-exercise-starter-expr` depends on it and owns concrete operations and binding. You will first
write the small cases by hand. Once their duplication is visible, generic unary, binary, and
ternary auto-vectorizers let a new expression author supply only one scalar operation.

![Map of the typed expression engine](./assets/map-of-types.svg)

The map has four reading directions:

1. `DataType` tells the planner what a value means; `PhysicalType` selects storage.
2. `Scalar`, `ScalarRef`, `Array`, and `ArrayBuilder` form one compile-time family, while erased
   enums cross runtime boundaries through checked conversions.
3. `ColumnViewImpl` normalizes array, constant, Indexed, and typed-null representations before one
   selected typed expression enters its row loop.
4. The facade depends on core, but core never depends on a concrete arithmetic, Boolean, or string
   operation. That one-way edge keeps the reusable loop independent of the function catalog.

The numeric chapters keep scalar hooks statically typed, auto-vectorize them through monomorphized
generic helpers, and erase only whole-batch adapters.
For signed addition, subtraction, and multiplication, `std::num::Wrapping<T>` makes the chosen
cross-profile overflow behavior explicit while still using the standard operator traits.

Nullability is value state—`Option` or validity—not a `DataType::Nullable` variant. A one-level
List adds offsets and independent outer/child validity; it does not add an aggregate engine.

## What you need to know

You should be comfortable with Rust enums, traits, references, `Option`, and ordinary Cargo use.
The course introduces generic associated types, checked runtime erasure, typestate, and
return-position `impl Trait` in the concrete places that need them.

Every chapter names prerequisites, exact starter targets, required work, extensions, and a copied
test. Passing the test is necessary; you should also be able to explain why the new boundary exists
and which failure it prevents.

Treat each of the fourteen focused chapters as roughly half a day. An experienced Rust learner can
finish the course in about seven working days; newer learners should expect to take longer.

Continue to [Environment Setup](./setup.md).

{{#include copyright.md}}
