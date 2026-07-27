# Draw the Framework Boundary

Part I identified a clean boundary between planning and execution. We can now write the contracts
before choosing the vectorized data structures.

## Planning Contract

The planner consumes a function name and logical input types. It returns a bound expression or a
specific error:

```rust,ignore
let bound = registry.bind_binary(name, left_type, right_type)?;
assert_eq!(bound.output_type(), &DataType::Boolean);
```

Planning owns overload selection, promotion policy, logical result types, and custom-function
validation.

## Execution Contract

The dyn-compatible runtime trait accepts type-erased logical columns and states its executor
thread-safety contract:

```rust,ignore
pub trait Expression: Send + Sync {
    fn eval(&self, data: &[ColumnViewImpl<'_>]) -> Result<ArrayImpl>;
}
```

The final code also retains the original array-only API as an adapter:

```rust,ignore
fn eval_expr(&self, data: &[&ArrayImpl]) -> Result<ArrayImpl>;
```

This adapter wraps each array in `ColumnViewImpl::Array`, so existing callers do not need an
immediate migration.

## Inside the Boundary

Once the type-erased views reach a generated expression, it performs three steps outside the hot
loop:

1. check and downcast each physical type to `ColumnView<'a, S>`;
2. match each storage representation (array, constant, or dictionary); and
3. call a monomorphized loop through `ColumnAccessor<'a, S>`.

Inside the loop, Rust sees concrete scalar, array, accessor, and function types. That is where the
type system earns its complexity. Outside the loop, the database retains a practical runtime API.

## What the Common Template Does Not Own

The common strict template does not attempt to solve:

- special null semantics;
- variable-length output that benefits from a specialized writer API;
- short-circuit boolean evaluation;
- selection vectors or sparse execution;
- expression-specific state such as compiled regular expressions; or
- logical rules for a new data type.

Those can still implement `Expression` or register a custom factory. Extensibility comes from a
small boundary, not from making one template understand every expression.

## Part I Checkpoint

At this point the architecture is logically complete but not efficient. A scalar evaluator can
produce correct results, and the binder can reject invalid programs. Part II replaces row values
with arrays and views without moving any type decision back into the per-row loop.

Continue to [the vectorized runtime](../vectorized/overview.md).

## Test Your Understanding

- Why is `Expression` dyn-compatible while `Array` will not be?
- Where does a runtime physical type check happen, and how often?
- How can a custom expression bypass strict null propagation without changing the registry API?

{{#include ../copyright.md}}
