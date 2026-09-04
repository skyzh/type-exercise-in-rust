{{#include wip-banner.md}}

# Chapter 2: Read Nullable Columns Lazily

An expression should be able to read a stored array, a repeated constant, an all-null value, or an
indexed projection through one logical-row interface. In this chapter you will build that borrowed
view without materializing a replacement array.

Copy the cumulative contract:

```console
cargo x copy-test --chapter 2
cargo test -p type-exercise-starter-supplied-tests chapter_2 --locked
```

Open `core/src/column.rs`. Implement `ColumnViewImpl<'a>` with checked constructors for four
behaviors:

```text
Array     -> read row directly from a borrowed ArrayImpl
Constant  -> repeat one borrowed scalar for len rows
Null      -> repeat no value while retaining a PhysicalType
Indexed   -> use each u32 key to read a borrowed values array
```

An empty or all-null column still needs a physical type. Later expression binding and output
allocation cannot infer that type from a non-null row that does not exist.

Validate every Indexed key in `ColumnViewImpl::indexed` before returning the view. Once a view is
constructed, `get(row)` should only perform the logical lookup; it should not discover a malformed
key halfway through evaluation.

## Check the scalar family once

Implement `ColumnView<'a, S>` and `TryFrom<ColumnViewImpl<'a>>`. The conversion checks the erased
physical type against `S::PHYSICAL_TYPE` once, then stores the correctly typed borrowed array,
constant, or indexed values. Its `get` returns `Option<S::RefType<'a>>` without an erased scalar
round trip on every row.

Keep this representation boundary small. Indexed is a borrowed execution view, not a persisted
dictionary encoding. Do not add key builders, expression loops, List handling, or specialization
yet.

Enable and export `column` from `core/src/lib.rs`, then run:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_2 --locked
cargo test -p type-exercise-checkpoint-02-supplied-tests --locked
cargo check -p type-exercise-checkpoint-02-core --locked
```

The checkpoint is complete when every representation yields the same logical rows, invalid keys
fail during construction, and a typed-family mismatch fails before row access.

[Chapter 3](./chapter-3-shared-evaluation.md) will put one reusable nullable loop over this view.

{{#include copyright.md}}
