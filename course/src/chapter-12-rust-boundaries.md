{{#include wip-banner.md}}

# Chapter 12: Build a One-Level List Column

A List column combines two independent structures: outer rows and one contiguous child array.
Offsets delimit each row's child slice, outer validity distinguishes null from valid rows, and the
child array retains its own physical family and validity. This chapter uses those invariants to
exercise ownership and checked erasure without building a general nested-type engine.

## The learner-owned boundary

```console
cargo x copy-test --chapter 12
cargo test -p type-exercise-starter-supplied-tests chapter_12 --locked
```

Implement `ListScalar`, borrowed `ListScalarRef`, `ListArray`, and `ListArrayBuilder` in the core
package. The course supports exactly one list level. Nested List construction must return a clear
error rather than recurse implicitly.

For `row_count = n`, preserve these invariants:

- `offsets.len() == n + 1`;
- offsets start at zero, never decrease, and end at the child length;
- a null outer row contributes no child values, so its adjacent offsets are equal;
- a valid empty row also has equal offsets but a true outer validity bit; and
- the declared child physical type matches the erased child array.

That final pair is why offsets alone cannot encode nullability.

## Build transactionally

When a row contains the wrong child type, return an error without publishing a partial List array.
Validate the complete row before advancing its final offset or validity bit. The private builder
rollback/state tests pin this internal atomicity; the learner-visible chapter test observes the
same guarantee through the public constructor.

Owned and borrowed list scalars should slice only their row's child range. They must not copy or
expose neighboring rows. Empty and all-null arrays still retain the declared child physical type.

## Erase and recover List views

Add List variants to the erased scalar/array/column families and implement checked recovery for
array, constant, null, and Indexed column views. A typed null must carry the List child family, so
recovering it as a different child type is rejected.

Run the full boundary:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_12 --locked
cargo test -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The nine focused tests cover logical/physical metadata, empty/null/nonempty rows, raw-part
validation, owned and borrowed slices, all erased view forms, typed-null mismatch, explicit nested
rejection, and failure without partial publication.

Next: [Chapter 13 strengthens Rust ownership boundaries](./chapter-13-async-boundary.md).

{{#include copyright.md}}
