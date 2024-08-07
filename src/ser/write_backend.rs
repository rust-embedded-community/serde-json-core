use super::{Result, Error, ser_backend::SerializerBackend};
use embedded_io::{Write, self};

pub struct WriteSerializer<'a, W: Write> {
    writer: &'a mut W,
    current_length: usize,
}

impl<'a, W: Write> WriteSerializer<'a, W> {
    /// Create a new `Serializer`
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            current_length: 0
        }
    }
}

impl<'a, W: Write> SerializerBackend for WriteSerializer<'a, W> {
    fn end(&self) -> usize {
        self.current_length
    }

    fn push(&mut self, c: u8) -> Result<()> {
        self.writer.write_all(&[c; 1]).map_err(|_err| Error::IOError)?;
        self.current_length = self.current_length + 1;
        Ok(())
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<()> {
        self.writer.write_all(other).map_err(|_err| Error::IOError)?;
        self.current_length = self.current_length + other.len();
        Ok(())
    }
}
