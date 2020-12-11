use serde::ser;

use crate::ser::{Error, Result, Serializer};

pub struct SerializeMap<'a, 'b> {
    ser: &'a mut Serializer<'b>,
    first: bool,
}

impl<'a, 'b: 'a> SerializeMap<'a, 'b> {
    pub(crate) fn new(ser: &'a mut Serializer<'b>) -> Self {
        SerializeMap { ser, first: true }
    }
}

impl<'a, 'b: 'a> ser::SerializeMap for SerializeMap<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn end(self) -> Result<Self::Ok> {
        self.ser.push(b'}')?;
        Ok(())
    }

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        if !self.first {
            self.ser.push(b',')?;
        }
        self.first = false;
        key.serialize(&mut *self.ser)?;
        self.ser.extend_from_slice(b":")?;
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
