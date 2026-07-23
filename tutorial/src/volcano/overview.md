# Part I: Start with a Scalar Evaluator

Before vectorizing expressions, build the smallest evaluator that has the right semantics. A
Volcano-style executor calls an operator for one tuple at a time. Its expression evaluator follows
the same shape: read one value from each input, call a scalar function, and return one value.

Suppose a plan contains:

```sql
price >= discount
```

A scalar evaluator might expose:

```rust
trait ScalarExpression {
    fn eval(&self, row: &[ScalarImpl]) -> Result<ScalarImpl>;
}
```

This interface is not our final implementation. It is valuable because all of the database
semantics are visible:

- input positions come from a row;
- each input has a logical SQL type;
- null propagation belongs to the expression contract;
- invalid signatures should have been rejected before `eval`; and
- the result is another database value.

## Planning and Execution Are Different Jobs

Do not make the evaluator search every overload for every row. Resolve the expression once:

```text
unbound call: >=(SmallInt, Double)
        |
        | binder chooses casts and implementation
        v
bound call: cmp_ge::<i16, f64, f64> -> Boolean
```

The bound call can trust its signature. Runtime data may still be type erased because storage and
network layers discover physical types dynamically, but the framework—not each function
author—performs the checked conversion.

This split becomes more important after vectorization. A batch may contain thousands of rows, so a
type decision made inside the loop is thousands of unnecessary branches.

## Why Not Begin with `dyn Scalar`?

Database values do not share one convenient borrowed representation. An `i32` is cheap to copy;
an Arrow-style string array returns `&str`; a list may return a view containing offsets and a
reference to another column. A trait object would either allocate owned values or expose a large
type-erased API.

We will instead use two layers:

1. concrete generic traits inside a kernel, where the compiler knows the types;
2. enums or object-safe traits at planning and execution boundaries, where the database knows types
   only at runtime.

## Part I Roadmap

The remaining Part I chapters build a semantic baseline:

1. define values, nulls, and chunks;
2. evaluate expressions one row at a time;
3. bind logical types and numeric promotion before execution; and
4. choose the exact boundary that the vectorized runtime must preserve.

The first design question is deceptively simple: [what is a database value?](./chunk.md)

## Test Your Understanding

- Which decisions can be made once per expression instead of once per row?
- Why can a query plan know `VARCHAR` while an array implementation only needs to know “string”?
- If a runtime downcast fails after binding, which layer has violated its contract?

{{#include ../copyright.md}}
