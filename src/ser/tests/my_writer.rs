pub struct MyWriter {
    pub buffer: [u8; 128],
    pub pos: usize,
    pub fail: bool
}

#[derive(Debug)]
pub struct MyWriterError { }

impl embedded_io::Error for MyWriterError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::OutOfMemory
    }
}

impl embedded_io::ErrorType for MyWriter {
    type Error = MyWriterError;
}

impl embedded_io::Write for MyWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let av = self.buffer.len() - self.pos;
        let wr = core::cmp::min(av, buf.len());
        self.buffer[self.pos..(self.pos + wr)].copy_from_slice(&buf[..wr]);
        self.pos = self.pos + wr;
        Ok(wr)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
