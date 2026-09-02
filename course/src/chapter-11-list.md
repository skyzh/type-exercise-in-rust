{{#include wip-banner.md}}

# Chapter 11: Bind and Coerce Logical Calls

The physical catalog can evaluate a known signature, but SQL starts with a logical name and
logical argument types. Binding is the planning step that resolves that call once. Evaluation must
not redo name lookup, promotion, or kernel selection for every batch or row.

## Checkpoint 1: store one finished binding

Start from completed Chapter 10:

```console
cargo x copy-test --chapter 11 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_11 --locked
```

Enable `expr/src/binder.rs`. Add the public binding error, `BoundExpression`, and
`FunctionRegistry` surfaces named by the starter. A bound expression stores the logical input and
output types beside one already-selected erased physical expression. Its constructor rejects
logical metadata that disagrees with that expression's physical input or output family.

A registry entry is a reusable factory over a complete logical input slice. `register` stores that
slice factory directly; the unary, binary, and ternary helpers check arity before adapting their
typed closures. `bind` resolves one name and calls its factory once. Unknown names and unsupported
arities fail without evaluating a batch.

The six focused tests keep this stage independent of the builtin catalog: they cover logical versus
physical metadata, zero through five input positions, custom registration, repeated binding, helper
arity, and unknown names. Passing them reaches 83 cumulative tests.

## Checkpoint 2: register the builtin logical catalog

```console
cargo x copy-test --chapter 11 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_11 --locked
```

Now implement `FunctionRegistry::with_builtins`. Register arithmetic, comparison, Boolean, and
string factories. Each factory either rejects its complete logical signature or returns one erased
physical expression whose metadata agrees with the requested logical call.

Binding follows this order:

1. resolve the logical function name;
2. check arity from the logical input slice;
3. apply explicit logical promotion rules where the function permits them;
4. choose one physical factory and validate its input/output metadata; and
5. store the finished expression in `BoundExpression`.

Evaluation then delegates directly to that expression.

## Arithmetic and comparisons

Register arithmetic and numeric comparison names through Chapter 5's promotion table. Reject
lossy pairs rather than inventing a cast. Arithmetic returns the promoted numeric type;
comparisons return Boolean while still comparing through the approved common family.

Keep operation names distinct. `add`, `subtract`, and `multiply` cannot share one accidental
default; the same applies to all six comparisons and their NaN behavior. The registry selects the
operation before the expression reaches a row loop.

## Boolean and strings

Register unary `not`, binary `and`/`or`, strict string comparisons, `contains`, and the Chapter 10
concatenation path with their exact logical signatures. Logical `Char` and `Varchar` may share a
physical representation without becoming the same planner type. Binding preserves that
distinction even though evaluation borrows the same UTF-8 slices.

## Complete contract

Run the final course boundary:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_11 --locked
cargo test -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The 19 focused tests reach 96 cumulative tests. They cover successful numeric, Boolean,
comparison, and string calls; unknown names; unsupported and lossy signatures; inconsistent
factory metadata; arbitrary arity slices; custom registration; checked runtime failures; and
metadata validation through both physical and bound expressions.

As a final inspection, confirm that binding chooses a factory before evaluation and that
`BoundExpression::evaluate` delegates the complete batch. Loop selection and the absence of a
facade row loop are implementation-review concerns; supplied tests grade only public API and
behavior.

The dependency direction remains important. The facade owns concrete operations and the builtin
registry. Core owns only the generic registry and erased expression vocabulary needed to store a
finished factory result. Core never imports the builtin catalog.

Next: [Chapter 12 builds a one-level List column](./chapter-12-rust-boundaries.md).

{{#include copyright.md}}
