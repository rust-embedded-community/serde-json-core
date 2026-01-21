use super::{Result, Error};

pub trait SerializerBackend {
    /// Return the current amount of serialized data in the buffer
    fn end(&self) -> usize;
    fn push(&mut self, c: u8) -> Result<()>;
    fn extend_from_slice(&mut self, other: &[u8]) -> Result<()>;
}

pub struct SliceSerializer<'a> {
    buf: &'a mut [u8],
    pub(crate) current_length: usize,
}

impl<'a> SliceSerializer<'a> {
    /// Create a new `Serializer`
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf,
            current_length: 0,
        }
    }

    unsafe fn push_unchecked(&mut self, c: u8) {
        self.buf[self.current_length] = c;
        self.current_length += 1;
    }
}

impl<'a> SerializerBackend for SliceSerializer<'a> {
    fn end(&self) -> usize {
        self.current_length
    }

    fn push(&mut self, c: u8) -> Result<()> {
        if self.current_length < self.buf.len() {
            unsafe { self.push_unchecked(c) };
            Ok(())
        } else {
            Err(Error::BufferFull)
        }
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<()> {
        if self.current_length + other.len() > self.buf.len() {
            // won't fit in the buf; don't modify anything and return an error
            Err(Error::BufferFull)
        } else {
            for c in other {
                unsafe { self.push_unchecked(*c) };
            }
            Ok(())
        }
    }
}
