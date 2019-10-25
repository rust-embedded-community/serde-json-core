use serde::ser;

use heapless::ArrayLength;

use crate::ser::{Error, Result, Serializer};

pub struct SerializeSeq<'a>  {
    de: &'a mut Serializer,
    first: bool,
}

impl<'a> SerializeSeq<'a> {
    pub(crate) fn new(de: &'a mut Serializer) -> Self {
        SerializeSeq { de, first: true }
    }
}

impl<'a> ser::SerializeSeq for SerializeSeq<'a>  {
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

impl<'a> ser::SerializeTuple for SerializeSeq<'a>  {
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
