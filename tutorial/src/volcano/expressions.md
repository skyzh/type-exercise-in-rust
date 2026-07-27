# Evaluate Expressions One Row at a Time

Start with a hand-written binary expression. This establishes the algorithm that the generated
vectorizer must preserve:

```rust
fn eval_i32_le(left: Option<i32>, right: Option<i32>) -> Option<bool> {
    left.zip(right).map(|(left, right)| left <= right)
}
```

For a row evaluator, the expression also has to locate its children:

```rust,ignore
enum Expr {
    InputRef(usize),
    Literal(ScalarImpl),
    Call {
        function: BoundFunction,
        children: Vec<Expr>,
    },
}
```

Evaluation recursively computes the children and invokes the already-bound function. Notice what
is absent: the evaluator does not compare SQL data types or search an overload table.

## Scalars and Columns Should Share a Kernel

A literal such as `price >= 100` is a scalar input next to a column input. A first vectorized design
might expand the literal into an array containing `100` thousands of times. That works, but it pays
allocation and memory bandwidth for a value already known to be constant.

The referenced typed expression project models an input as either a scalar or a column and writes
four binary cases:

```text
scalar / scalar
scalar / column
column / scalar
column / column
```

This course generalizes that useful idea into a column view. A constant is a logical column whose
`get(row)` always returns the same borrowed scalar. The generated kernel therefore has one loop for
all cases instead of four copies of its null and output-building logic.

## Dictionary Inputs Reveal the Same Pattern

Suppose a string column contains many repeated countries:

```text
dictionary values: ["US", "CN", "JP"]
row indices:       [0, 0, 1, NULL, 2, 0]
```

Materializing six strings before evaluating `country = 'US'` is unnecessary. A dictionary view
maps the row index to the values array when the kernel asks for a value. The scalar function still
receives `&str`.

## The Baseline Loop

Regardless of encoding, a strict binary expression has this logical algorithm:

```rust,ignore
let mut output = O::Builder::with_capacity(len);
for row in 0..len {
    match (left.get(row), right.get(row)) {
        (Some(left), Some(right)) => {
            output.push(Some(func(left, right).as_scalar_ref()));
        }
        _ => output.push(None),
    }
}
output.finish()
```

Part II will make every type in this pseudocode precise. Before that, the planner must guarantee
that `left`, `right`, and `func` agree.

## Task

Write scalar pseudocode for these expressions and identify their null policy:

1. integer addition;
2. string `contains`;
3. `IS NULL`; and
4. `COALESCE(left, right)`.

Only the first two fit the strict loop unchanged. This is why the final framework provides a common
template without claiming that every expression must use it.

Continue to [logical type binding](./data_types.md).

{{#include ../copyright.md}}
