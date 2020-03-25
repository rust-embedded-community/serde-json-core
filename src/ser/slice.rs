pub trait MutSlice {
    type Output;

    fn push(&mut self, b: u8) -> Result<(), u8>;
    fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), ()>;
    fn release(self) -> Self::Output;
}

pub struct Slice<'a> {
    buf: &'a mut [u8],
    index: usize,
}

impl<'a> Slice<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Slice { buf, index: 0 }
    }
}

impl<'a> MutSlice for Slice<'a> {
    type Output = &'a mut [u8];
    fn push(&mut self, b: u8) -> Result<(), u8> {
        if self.index >= self.buf.len() {
            return Err(b);
        }

        self.buf[self.index] = b;
        self.index += 1;
        Ok(())
    }

    fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), ()> {
        if self.index + slice.len() >= self.buf.len() {
            return Err(());
        }
        self.buf[self.index..self.index + slice.len()].copy_from_slice(slice);
        self.index += slice.len();
        Ok(())
    }

    fn release(self) -> Self::Output {
        let (used, _unused) = self.buf.split_at_mut(self.index);

        used
    }
}

use heapless::Vec;

pub struct VecSlice<B: heapless::ArrayLength<u8>>(pub Vec<u8, B>);

impl<B> VecSlice<B>
where
    B: heapless::ArrayLength<u8>,
{
    pub fn new() -> Self {
        VecSlice(Vec::new())
    }
}

impl<B: heapless::ArrayLength<u8>> MutSlice for VecSlice<B> {
    type Output = Vec<u8, B>;
    fn push(&mut self, b: u8) -> Result<(), u8> {
        self.0.push(b)
    }

    fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), ()> {
        self.0.extend_from_slice(slice)
    }
    fn release(self) -> Self::Output {
        self.0
    }
}
