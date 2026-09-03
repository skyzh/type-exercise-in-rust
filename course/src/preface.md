{{#include wip-banner.md}}

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

Each lab begins from the preceding completed snapshot, names the learner-owned change, and gives an
exact command for useful feedback. Passing the supplied test is necessary; you should also be able
to explain why the new boundary exists and which failure it prevents.

The fourteen labs form seven modules:

1. **Type families** (Chapters 1–2) connects scalar and array representations, then scales the
   finite catalog.
2. **Borrowed columns and first batch evaluation** (Chapters 3–4) normalizes column
   representations and lifts the first scalar operation over a batch.
3. **Generic numeric evaluation** (Chapters 5–6) separates promotion, kernel selection, and
   reusable arity-shaped vectorization.
4. **Specialized execution and Boolean nulls** (Chapters 7–8) adds a narrow dense path and then
   handles SQL's non-strict Boolean semantics.
5. **Runtime expressions and variable-width output** (Chapters 9–10) erases whole expressions
   and publishes string rows transactionally.
6. **Logical binding and nested storage** (Chapters 11–12) resolves runtime calls and extends the
   same boundaries to one-level Lists.
7. **Thread-safe and async boundaries** (Chapters 13–14) makes logical factories shareable and
   wraps one batch in a future without introducing per-row async work.

Treat each lab as roughly half a day. An experienced Rust learner can finish the course in about
seven working days; newer learners should expect to take longer.

Continue to [Environment Setup](./setup.md).

{{#include copyright.md}}
