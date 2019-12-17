use serde::ser;

use heapless::ArrayLength;

use crate::ser::{Error, Result, Serializer};

pub struct SerializeMap<'a, B>
where
    B: ArrayLength<u8>,
{
    ser: &'a mut Serializer<B>,
    first: bool,
}

impl<'a, B> SerializeMap<'a, B>
where
    B: ArrayLength<u8>,
{
    pub(crate) fn new(ser: &'a mut Serializer<B>) -> Self {
        SerializeMap { ser, first: true }
    }
}

impl<'a, B> ser::SerializeMap for SerializeMap<'a, B>
where
    B: ArrayLength<u8>,
{
    type Ok = ();
    type Error = Error;

    fn end(self) -> Result<Self::Ok> {
        self.ser.buf.push(b'}')?;
        Ok(())
    }

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        if !self.first {
            self.ser.buf.push(b',')?;
        }
        self.first = false;
        key.serialize(&mut *self.ser)?;
        self.ser.buf.extend_from_slice(b":")?;
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        value.serialize(&mut *self.ser)?;
        Ok(())
    }
}
