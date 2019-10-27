use serde::ser;

use crate::ser::{Error, Result, Serializer};

pub struct SerializeStruct<'a> {
    de: &'a mut Serializer,
    first: bool,
}

impl<'a> SerializeStruct<'a> {
    pub(crate) fn new(de: &'a mut Serializer) -> Self {
        SerializeStruct { de, first: true }
    }
}

impl<'a> ser::SerializeStruct for SerializeStruct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        // XXX if `value` is `None` we not produce any output for this field
        if !self.first {
            self.de.buf.push(b',');
        }
        self.first = false;

        self.de.buf.push(b'"');
        self.de.buf.extend_from_slice(key.as_bytes());
        self.de.buf.extend_from_slice(b"\":");

        value.serialize(&mut *self.de)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        self.de.buf.push(b'}');
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for SerializeStruct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        // XXX if `value` is `None` we not produce any output for this field
        if !self.first {
            self.de.buf.push(b',');
        }
        self.first = false;

        self.de.buf.push(b'"');
        self.de.buf.extend_from_slice(key.as_bytes());
        self.de.buf.extend_from_slice(b"\":");

        value.serialize(&mut *self.de)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        // close struct
        self.de.buf.push(b'}');
        // close surrounding enum
        self.de.buf.push(b'}');
        Ok(())
    }
}
