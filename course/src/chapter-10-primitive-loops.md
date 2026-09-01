{{#include wip-banner.md}}

# Chapter 10: Build Variable-Width Strings Transactionally

Fixed-width evaluators can compute a scalar and then push its copied value. A string result has no
fixed-size scalar to return: its UTF-8 bytes and ending offset must be appended together, and the
validity buffer must gain exactly one row. Publishing only part of that state would corrupt the
array.

This chapter introduces variable-width output only after the fixed-width evaluator model is
settled. A consumed writer typestate makes one successful callback correspond to one complete row.

## Checkpoint 1: pin the physical representation

```console
cargo x copy-test --chapter 10 --checkpoint 1
cargo test -p type-exercise-starter-supplied-tests chapter_10 --locked
```

Review the Chapter 1 `StringArray` representation:

- `data` is one contiguous UTF-8 byte buffer;
- `offsets` has one more entry than the row count, and row `i` uses
  `data[offsets[i]..offsets[i + 1]]`;
- `validity` has one bit per logical row; and
- a null row repeats the previous offset because it contributes no bytes.

The first test pins those buffers directly. It distinguishes an empty string—a valid row whose
two offsets are equal—from a null row with the same byte span but a false validity bit.

## Checkpoint 2: consume the writer exactly once

```console
cargo x copy-test --chapter 10 --checkpoint 2
cargo test -p type-exercise-starter-supplied-tests chapter_10 --locked
```

Add `Writer<'a>` and `WriterUsed<'a>` around a borrowed `StringArrayBuilder`. The public operation
is shaped like this:

```rust,ignore
impl<'a> Writer<'a> {
    pub fn write(self, value: &str) -> WriterUsed<'a>;
}
```

`write` consumes the unused writer, appends the bytes, ending offset, and valid bit, then returns
proof that this row has been published. Only the core evaluator may recover the builder from
`WriterUsed` to begin the next row.

The type transition prevents a callback from returning without publishing or calling `write`
twice through the same value. It does not make partial mutation magically reversible; instead, the
facade operation must prepare any fallible work before it consumes the writer. This is why the
public callback has no path that returns an arbitrary builder.

## Checkpoint 3: lift borrowed string work without allocation

```console
cargo x copy-test --chapter 10 --checkpoint 3
cargo test -p type-exercise-starter-supplied-tests chapter_10 --locked
cargo check -p type-exercise-starter-core --locked
```

Enable `expr/src/string.rs` and implement the borrowed concatenation scalar operation. It receives two
`&str` values plus a `Writer`, writes each input directly into the builder's bytes, publishes one
offset and validity bit, and returns `WriterUsed`. Do not allocate a temporary `String` with
`format!`, and do not write an operation-specific batch loop.

The core variable-width evaluator validates and recovers both typed string views once, propagates
strict nulls itself, and constructs a fresh writer only for a non-null row. Array, constant, and
Indexed inputs therefore share the same borrowed scalar operation.

The three focused tests pin the physical bytes/offsets/validity layout, the consumed public writer
surface, and concatenation across borrowed representations. Core still compiles independently of
the concrete concatenation function.

## The ownership proof

`Writer<'a> -> WriterUsed<'a>` is a small typestate machine. The lifetime keeps both values tied to
the evaluator's builder; the move prevents reuse; and the distinct return type proves a row was
published before the evaluator continues. The compiler checks the transaction boundary that a
runtime `wrote: bool` flag would only check after the fact.

Next: [Chapter 11 binds logical calls to physical expressions](./chapter-11-list.md).

{{#include copyright.md}}
