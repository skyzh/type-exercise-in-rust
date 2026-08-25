/// Day 1, checkpoint 3: replace this shell with shared UTF-8 bytes, offsets, and packed validity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StringArray;

#[derive(Clone, Debug, Default, PartialEq)]
/// Day 1, checkpoint 3: replace this shell with append-only UTF-8/offset/validity buffers.
pub struct StringArrayBuilder;

// Day 1, checkpoint 3: add `data`/`offsets`/`validity`, implement Array for StringArray, and
// implement its builder. String reads borrow `&str` directly from the shared byte buffer.
