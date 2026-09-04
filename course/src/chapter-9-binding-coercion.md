{{#include wip-banner.md}}

# Chapter 9: Build a One-Level List Column

A List column combines outer rows with one contiguous child array. Offsets delimit each row's child
slice; outer validity distinguishes null from valid-empty; and the child array keeps its own
physical family and nulls. This course supports exactly one List level.

```console
cargo x copy-test --chapter 9
cargo test -p type-exercise-starter-supplied-tests chapter_9 --locked
```

## Preserve the three independent invariants

Add `List` to `DataType`, `PhysicalType`, `ScalarImpl`, `ScalarRefImpl`, and `ArrayImpl`. The type
stores the child family. An empty or all-null outer array must still retain that family.

Implement owned `ListScalar`, borrowed `ListScalarRef`, `ListArray`, and its builder. For `n`
outer rows:

- offsets have length `n + 1`, start at zero, never decrease, and end at the child length;
- null outer rows contribute no children and repeat the previous offset;
- valid empty rows have the same equal offsets but a true validity bit; and
- every appended child slice has the declared physical family.

Validate one complete row before advancing its offset or validity. Reject a nested List child
explicitly instead of recursing into a type system the course has not defined.

Extend `ColumnViewImpl` with checked `try_as_list`. Array, Constant, typed null, and Indexed forms
must preserve the child family and borrow only the selected row's child slice. Slicing rebases
offsets and cannot expose neighboring rows.

```console
cargo test -p type-exercise-starter-supplied-tests chapter_9 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
```

The tests cover null/empty/nonempty rows, child nulls, slicing, all four column representations,
wrong child types, invalid raw parts, and explicit nested rejection. [Chapter 10](./chapter-10-primitive-loops.md)
keeps these borrows intact across thread-safe factories and one batch future.

{{#include copyright.md}}
