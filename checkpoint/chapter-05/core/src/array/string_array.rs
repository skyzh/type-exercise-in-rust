use crate::{Array, ArrayBuilder};
use bitvec::vec::BitVec;

/// A nullable UTF-8 array backed by bytes, offsets, and a packed validity bitmap.
#[derive(Clone, Debug, PartialEq)]
pub struct StringArray {
    data: Vec<u8>,
    offsets: Vec<usize>,
    validity: BitVec,
}

/// The append-only builder for [`StringArray`].
#[derive(Debug)]
pub struct StringArrayBuilder {
    data: Vec<u8>,
    offsets: Vec<usize>,
    validity: BitVec,
}

/// A transactional view of the byte range for one pending string value.
pub struct StringValueWriter<'a> {
    data: &'a mut Vec<u8>,
}

/// An unpublished output row. Consuming [`Writer::write`] is the only way to
/// turn it into [`WriterUsed`].
///
/// ```compile_fail
/// # use type_exercise_checkpoint_05_core::{Writer, WriterUsed};
/// fn skip_write(writer: Writer<'_>) -> WriterUsed<'_> {
///     writer
/// }
/// ```
pub struct Writer<'a> {
    builder: &'a mut StringArrayBuilder,
}

/// Proof that one output row was written exactly once.
///
/// ```compile_fail
/// # use type_exercise_checkpoint_05_core::Writer;
/// fn write_twice(writer: Writer<'_>) {
///     let writer = writer.write(|value| value.push_str("first"));
///     let _ = writer.write(|value| value.push_str("second"));
/// }
/// ```
pub struct WriterUsed<'a> {
    builder: &'a mut StringArrayBuilder,
}

impl StringValueWriter<'_> {
    /// Append one UTF-8 fragment directly to the pending value.
    pub fn push_str(&mut self, value: &str) {
        self.data.extend_from_slice(value.as_bytes());
    }
}

impl<'a> Writer<'a> {
    /// Publish exactly one non-null row, possibly from several UTF-8 fragments.
    pub fn write(self, write: impl FnOnce(&mut StringValueWriter<'_>)) -> WriterUsed<'a> {
        self.builder
            .try_push_with(|value| {
                write(value);
                Ok::<_, std::convert::Infallible>(())
            })
            .unwrap_or_else(|never| match never {});
        WriterUsed {
            builder: self.builder,
        }
    }
}

impl<'a> WriterUsed<'a> {
    pub(crate) fn into_builder(self) -> &'a mut StringArrayBuilder {
        self.builder
    }
}

impl StringArrayBuilder {
    pub(crate) fn writer(&mut self) -> Writer<'_> {
        Writer { builder: self }
    }
    /// Append a null row without changing the shared byte buffer.
    pub fn push_null(&mut self) {
        self.validity.push(false);
        self.offsets.push(self.data.len());
    }

    /// Build one non-null value in place, publishing its row metadata only on success.
    pub fn try_push_with<E>(
        &mut self,
        write: impl FnOnce(&mut StringValueWriter<'_>) -> Result<(), E>,
    ) -> Result<(), E> {
        let start = self.data.len();
        let result = write(&mut StringValueWriter {
            data: &mut self.data,
        });
        match result {
            Ok(()) => {
                self.validity.push(true);
                self.offsets.push(self.data.len());
                Ok(())
            }
            Err(error) => {
                self.data.truncate(start);
                Err(error)
            }
        }
    }
}

impl StringArray {
    /// The contiguous UTF-8 byte buffer.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// The row boundaries into [`Self::data`].
    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    /// The packed row-validity bitmap.
    pub fn validity(&self) -> &BitVec {
        &self.validity
    }
}

impl Array for StringArray {
    type Builder = StringArrayBuilder;
    type OwnedItem = String;
    type RefItem<'a> = &'a str;

    fn get(&self, row: usize) -> Option<Self::RefItem<'_>> {
        if !self.validity[row] {
            return None;
        }

        let bytes = &self.data[self.offsets[row]..self.offsets[row + 1]];
        Some(std::str::from_utf8(bytes).expect("StringArrayBuilder accepts only UTF-8 strings"))
    }

    fn len(&self) -> usize {
        self.validity.len()
    }
}

impl ArrayBuilder for StringArrayBuilder {
    type Array = StringArray;

    fn with_capacity(capacity: usize) -> Self {
        let mut offsets = Vec::with_capacity(capacity + 1);
        offsets.push(0);
        Self {
            data: Vec::new(),
            offsets,
            validity: BitVec::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: Option<&str>) {
        match value {
            Some(value) => self
                .try_push_with(|writer| {
                    writer.push_str(value);
                    Ok::<_, std::convert::Infallible>(())
                })
                .unwrap_or_else(|never| match never {}),
            None => self.push_null(),
        }
    }

    fn finish(self) -> Self::Array {
        StringArray {
            data: self.data,
            offsets: self.offsets,
            validity: self.validity,
        }
    }
}
