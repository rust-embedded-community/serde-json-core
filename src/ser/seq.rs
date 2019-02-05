use serde::ser;

use ser::{Error, Result, Serializer};

pub struct SerializeSeq<'a, B>
where
    B: heapless::ArrayLength<u8> + 'a,
{
    de: &'a mut Serializer<B>,
    first: bool,
}

impl<'a, B> SerializeSeq<'a, B>
where
    B: heapless::ArrayLength<u8>,
{
    pub(crate) fn new(de: &'a mut Serializer<B>) -> Self {
        SerializeSeq { de, first: true }
    }
}

impl<'a, B> ser::SerializeSeq for SerializeSeq<'a, B>
where
    B: heapless::ArrayLength<u8>,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        if !self.first {
            self.de.buf.push(b',')?;
        }
        self.first = false;

        value.serialize(&mut *self.de)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        self.de.buf.push(b']')?;
        Ok(())
    }
}

impl<'a, B> ser::SerializeTuple for SerializeSeq<'a, B>
where
    B: heapless::ArrayLength<u8>,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}
