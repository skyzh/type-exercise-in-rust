{{#include wip-banner.md}}

# Chapter 11: Bind and Coerce Logical Calls

The physical catalog can evaluate a known signature, but SQL starts with a logical name and
logical argument types. Binding is the planning step that resolves that call once. Evaluation must
not redo name lookup, promotion, or kernel selection for every batch or row.

## The learner-owned boundary

Start from completed Chapter 10:

```console
cargo x copy-test --chapter 11
cargo test -p type-exercise-starter-expr chapter_11 --locked
```

Enable `src/binder.rs` and implement the public `FunctionRegistry`, `BoundExpression`, and binding
errors. A registry entry is a factory over a complete logical input slice, not a binary-only
closure. The factory either rejects the signature or returns one erased physical expression whose
metadata agrees with the requested logical call.

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
concatenation path with their exact logical signatures. Logical `String` and `Varchar` may share a
physical representation without becoming the same planner type. Binding preserves that
distinction even though evaluation borrows the same UTF-8 slices.

## Complete contract

Run the final course boundary:

```console
cargo test -p type-exercise-starter-expr chapter_11 --locked
cargo test -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The 19 focused tests cover successful numeric, Boolean, comparison, and string calls; unknown
names; unsupported and lossy signatures; inconsistent factory metadata; arbitrary arity slices;
custom registration; checked runtime errors; and nullability propagation through both physical
and bound expressions.

The dependency direction remains important. The facade owns concrete operations and the builtin
registry. Core owns only the generic registry and erased expression vocabulary needed to store a
finished factory result. Core never imports the builtin catalog.

Next: [Chapter 12 builds a one-level List column](./chapter-12-rust-boundaries.md).

{{#include copyright.md}}
