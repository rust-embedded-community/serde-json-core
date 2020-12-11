use serde::ser;

use crate::ser::{Error, Result, Serializer};

pub struct SerializeStruct<'a, 'b> {
    ser: &'a mut Serializer<'b>,
    first: bool,
}

impl<'a, 'b: 'a> SerializeStruct<'a, 'b> {
    pub(crate) fn new(ser: &'a mut Serializer<'b>) -> Self {
        SerializeStruct { ser, first: true }
    }
}

impl<'a, 'b: 'a> ser::SerializeStruct for SerializeStruct<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize,
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

    fn end(self) -> Result<Self::Ok> {
        self.ser.push(b'}')?;
        Ok(())
    }
}

pub struct SerializeStructVariant<'a, 'b> {
    ser: &'a mut Serializer<'b>,
    first: bool,
}

impl<'a, 'b: 'a> SerializeStructVariant<'a, 'b> {
    pub(crate) fn new(ser: &'a mut Serializer<'b>) -> Self {
        SerializeStructVariant { ser, first: true }
    }
}

impl<'a, 'b: 'a> ser::SerializeStructVariant for SerializeStructVariant<'a, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ser::Serialize,
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

    fn end(self) -> Result<Self::Ok> {
        self.ser.extend_from_slice(b"}}")?;
        Ok(())
    }
}
