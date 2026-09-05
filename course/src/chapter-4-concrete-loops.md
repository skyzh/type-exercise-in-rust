# Checkpoint 4: Build Variable-Width Rows Transactionally

Checkpoint 3 can lift a scalar function when one owned value represents an output row. A string
row is different: its UTF-8 bytes, terminal offset, and validity bit must become visible together.
This checkpoint makes that publication boundary explicit.

Start from your completed Checkpoint 3 workspace, copy the cumulative tests, and run the focused
test once before editing:

```console
cargo x copy-test --chapter 4
cargo test -p type-exercise-starter-supplied-tests chapter_4 --locked
```

The first run should fail on the missing writer surface. Leave later shape specialization and
runtime expression types absent.

## Consume the only unpublished handle

In `core/src/array/string_array.rs`, add `Writer<'a>` and `WriterUsed<'a>` around a borrowed
`StringArrayBuilder`. The transition has this shape:

```rust,ignore
impl<'a> Writer<'a> {
    pub fn write(
        self,
        write: impl FnOnce(&mut StringValueWriter<'_>),
    ) -> WriterUsed<'a>;
}
```

The closure may append several borrowed fragments. A successful call then commits one terminal
offset and one true validity bit. A null row appends no bytes, repeats the terminal offset, and
commits one false bit. If a fallible builder callback stops before publication, truncate the bytes
back to their starting length and leave offsets and validity unchanged.

Consuming `Writer` prevents a scalar callback from skipping publication or publishing twice. Only
the core evaluator may recover the builder from `WriterUsed` and begin the next row.

## Lift one borrowed string operation

Add `evaluate_writer_binary` to `core/src/expression.rs`. It validates two String inputs and their
lengths before reading a row, converts both inputs to typed borrowed views once, and owns the only
batch loop. For a non-null pair it passes `&str`, `&str`, and a fresh `Writer` to the callback. For
a null pair it publishes one null directly.

Keep this boundary in core. The supplied test passes its borrowed concatenation callback directly;
do not enable the concrete String facade yet.

The callback can concatenate two borrowed strings without allocating a temporary `String`:

```rust,ignore
|left, right, writer| writer.write(|value| {
    value.push_str(left);
    value.push_str(right);
})
```

Run the focused and cumulative contracts:

```console
cargo test -p type-exercise-starter-supplied-tests chapter_4 --locked
cargo test -p type-exercise-starter-supplied-tests --lib --locked
```

The two Checkpoint 4 tests distinguish empty from null strings, pin bytes and offsets, and prove
failed writes do not leak partial bytes. Together with Checkpoints 1–3, the cumulative suite has 14
tests. Shape specialization, semantic exceptions, concrete String factories, runtime erasure, and
the registry remain future work.

{{#include copyright.md}}
