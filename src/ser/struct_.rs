use core::fmt::Display;

use embedded_io::Write;
use serde::ser;

use crate::ser::{Error, Serializer};

pub struct SerializeStruct<'a, W> {
    ser: &'a mut Serializer<W>,
    first: bool,
}

impl<'a, W> SerializeStruct<'a, W> {
    pub(crate) fn new(ser: &'a mut Serializer<W>) -> Self {
        SerializeStruct { ser, first: true }
    }
}

impl<'a, W: Write<Error: Display>> ser::SerializeStruct for SerializeStruct<'a, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        // XXX if `value` is `None` we not produce any output for this field
        if !self.first {
            self.ser.push(b',')?;
        }
        self.first = false;

        self.ser.push(b'"')?;
        self.ser.extend_from_slice(key.as_bytes())?;
        self.ser.extend_from_slice(b"\":")?;

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.push(b'}')?;
        Ok(())
    }
}

pub struct SerializeStructVariant<'a, W> {
    ser: &'a mut Serializer<W>,
    first: bool,
}

impl<'a, W> SerializeStructVariant<'a, W> {
    pub(crate) fn new(ser: &'a mut Serializer<W>) -> Self {
        SerializeStructVariant { ser, first: true }
    }
}

impl<'a, W: Write<Error: Display>> ser::SerializeStructVariant for SerializeStructVariant<'a, W> {
    type Ok = ();
    type Error = Error<W::Error>;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        // XXX if `value` is `None` we not produce any output for this field
        if !self.first {
            self.ser.push(b',')?;
        }
        self.first = false;

        self.ser.push(b'"')?;
        self.ser.extend_from_slice(key.as_bytes())?;
        self.ser.extend_from_slice(b"\":")?;

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.extend_from_slice(b"}}")?;
        Ok(())
    }
}
