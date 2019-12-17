use serde::ser;

use heapless::ArrayLength;

use crate::ser::{Error, Result, Serializer};

pub struct SerializeStruct<'a, B>
where
    B: ArrayLength<u8>,
{
    ser: &'a mut Serializer<B>,
    first: bool,
}

impl<'a, B> SerializeStruct<'a, B>
where
    B: ArrayLength<u8>,
{
    pub(crate) fn new(ser: &'a mut Serializer<B>) -> Self {
        SerializeStruct { ser, first: true }
    }
}

impl<'a, B> ser::SerializeStruct for SerializeStruct<'a, B>
where
    B: ArrayLength<u8>,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        // XXX if `value` is `None` we not produce any output for this field
        if !self.first {
            self.ser.buf.push(b',')?;
        }
        self.first = false;

        self.ser.buf.push(b'"')?;
        self.ser.buf.extend_from_slice(key.as_bytes())?;
        self.ser.buf.extend_from_slice(b"\":")?;

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        self.ser.buf.push(b'}')?;
        Ok(())
    }
}
