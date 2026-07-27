# Part II: Build the Vectorized Runtime

A vectorized engine evaluates one expression over a batch of values. The scalar function remains
small:

```rust,ignore
fn str_contains(left: &str, right: &str) -> bool {
    left.contains(right)
}
```

The framework supplies iteration, null propagation, output allocation, logical-to-physical
binding, and runtime dispatch.

## The Complete Data Path

For `country = 'US'`, execution looks like this:

```text
DataType::Varchar + DataType::Varchar
                |
                | bind once
                v
BinaryExpression<String, String, bool, cmp_eq>
                |
                | erase for the plan tree
                v
Box<dyn Expression>
                |
                | evaluate dictionary + constant views
                v
ColumnView<String> ---- ColumnView<String>
                |
                | dispatch encoding once
                v
DictionaryAccessor ---- ConstantAccessor
                |
                | loop over borrowed &str values
                v
BoolArray
```

The design has static dispatch inside the loop and dynamic dispatch around the batch. That balance
is more important than eliminating every enum or trait object in the framework.

## Build Order

Part II follows the dependencies in the implementation:

1. `Array` and `ArrayBuilder` define physical storage.
2. `Scalar` and `ScalarRef` connect owned and borrowed values to arrays.
3. `ColumnView` abstracts physical encodings without hiding the scalar type.
4. `ArrayImpl` and `ScalarRefImpl` erase types at runtime boundaries.
5. generated templates vectorize ordinary functions of one through five arguments.
6. macros expand numeric/comparison families while explicit kernels handle custom types.
7. the registry binds a logical call to the final runtime object.

Each layer has one direction of dependency. Arrays do not know expression names. Scalar kernels do
not know `DataType`. The binder does not know array buffer details. This separation keeps the type
relationships understandable.

## Correctness Invariants

Keep these invariants in mind:

- all input views to one expression have equal logical length;
- an array's `Builder` must finish back into that same array type;
- a scalar's borrowed representation must match the value returned by its array;
- a dictionary key is either null or in bounds;
- the binder's physical input type must match the runtime view; and
- strict expressions produce null if any input at that row is null.

The next chapter starts at the bottom with [Arrow-like arrays](./array.md).

{{#include ../copyright.md}}
