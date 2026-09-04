/// Chapter 1: replace this shell with shared UTF-8 bytes, offsets, and packed validity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StringArray;

#[derive(Clone, Debug, Default, PartialEq)]
/// Chapter 1: replace this shell with append-only UTF-8/offset/validity buffers.
pub struct StringArrayBuilder;

// Chapter 1: add `data`/`offsets`/`validity`, implement Array for StringArray, and
// implement its builder. String reads borrow `&str` directly from the shared byte buffer.

// Chapter 4: add `Writer<'a>` and `WriterUsed<'a>`. `Writer::write(self, ...)`
// consumes the unused writer and returns proof of exactly one published string row; only the
// evaluator may recover the borrowed builder from `WriterUsed` for the next row.
