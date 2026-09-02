{{#include wip-banner.md}}

# Chapter 12: Build a One-Level List Column

A List column combines two independent structures: outer rows and one contiguous child array.
Offsets delimit each row's child slice, outer validity distinguishes null from valid rows, and the
child array retains its own physical family and validity. This chapter uses those invariants to
exercise ownership and checked erasure without building a general nested-type engine.

## Checkpoint 1: name List and own one child slice

```console
cargo x copy-test --chapter 12 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_12 --locked
```

Add the `List` logical and physical metadata that preserves the element's physical family. Then
implement owned `ListScalar` and borrowed `ListScalarRef` in the core package. Both forms own or
borrow exactly one child-array range; slicing must not copy or expose neighboring items, and an
out-of-bounds range must fail.

The two focused tests cover the logical-to-physical mapping and owned/borrowed slicing. Passing
them reaches 98 cumulative tests.

## Checkpoint 2: build outer storage transactionally

```console
cargo x copy-test --chapter 12 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_12 --locked
```

Implement `ListArray` and `ListArrayBuilder`, then add List to the erased scalar and array
families. The course supports exactly one list level. Nested List construction must return an
error rather than recurse implicitly.

For `row_count = n`, preserve these invariants:

- `offsets.len() == n + 1`;
- offsets start at zero, never decrease, and end at the child length;
- a null outer row contributes no child values, so its adjacent offsets are equal;
- a valid empty row also has equal offsets but a true outer validity bit; and
- the declared child physical type matches the erased child array.

That final pair is why offsets alone cannot encode nullability.

When a row contains the wrong child type, return an error without publishing a partial List array.
Validate the complete row before advancing its final offset or validity bit. The private builder
state is an implementation detail; the learner-visible test observes the transaction through the
public constructor.

Owned and borrowed list scalars should slice only their row's child range. They must not copy or
expose neighboring rows. Empty and all-null arrays still retain the declared child physical type.

The seven focused tests cover metadata, owned/borrowed slices, empty/null/nonempty rows, raw-part
validation, explicit nested rejection, and failure without partial publication. Passing them
reaches 103 cumulative tests.

## Checkpoint 3: erase and recover List views

```console
cargo x copy-test --chapter 12 --checkpoint 3
cargo test -p type-exercise-starter-supplied-tests chapter_12 --locked
```

Implement checked List recovery for array, constant, null, and Indexed column views. A typed null
must carry the List child family, so recovering it as a different child type is rejected. Reject a
nested List child before exposing a typed List view.

Run the full boundary:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_12 --locked
cargo test -p type-exercise-starter-expr --lib --locked
cargo check -p type-exercise-starter-core --locked
```

The nine focused tests reach 105 cumulative tests. They cover logical/physical metadata,
empty/null/nonempty rows, raw-part validation, owned and borrowed slices, all erased view forms,
typed-null mismatch, explicit nested rejection, and failure without partial publication.

Next: [Chapter 13 strengthens Rust ownership boundaries](./chapter-13-async-boundary.md).

{{#include copyright.md}}
