use core::fmt::Display;

use embedded_io::Write;
use serde::ser;

use crate::ser::{Error, Serializer};

pub struct SerializeSeq<'a, W> {
    de: &'a mut Serializer<W>,
    first: bool,
}

impl<'a, W> SerializeSeq<'a, W> {
    pub(crate) fn new(de: &'a mut Serializer<W>) -> Self {
        SerializeSeq { de, first: true }
    }
}

impl<'a, W: Write<Error: Display>> ser::SerializeSeq for SerializeSeq<'a, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        if !self.first {
            self.de.push(b',')?;
        }
        self.first = false;

        value.serialize(&mut *self.de)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.de.push(b']')?;
        Ok(())
    }
}

impl<'a, W: Write<Error: Display>> ser::SerializeTuple for SerializeSeq<'a, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W: Write<Error: Display>> ser::SerializeTupleStruct for SerializeSeq<'a, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}
