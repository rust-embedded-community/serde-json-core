use core::fmt::Display;

use embedded_io::Write;
use serde::ser;

use crate::ser::{Error, Serializer};

pub struct SerializeMap<'a, W> {
    ser: &'a mut Serializer<W>,
    first: bool,
}

impl<'a, W> SerializeMap<'a, W> {
    pub(crate) fn new(ser: &'a mut Serializer<W>) -> Self {
        SerializeMap { ser, first: true }
    }
}

impl<'a, W: Write<Error: Display>> ser::SerializeMap for SerializeMap<'a, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.push(b'}')?;
        Ok(())
    }

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        if !self.first {
            self.ser.push(b',')?;
        }
        self.first = false;
        key.serialize(&mut *self.ser)?;
        self.ser.extend_from_slice(b":")?;
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(&mut *self.ser)?;
        Ok(())
    }
}
