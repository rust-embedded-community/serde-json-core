//! Serialize a Rust data structure into JSON data

use core::{fmt, fmt::Write};

use serde::ser;

use heapless::{consts::*, String, Vec};

use self::map::SerializeMap;
use self::seq::SerializeSeq;
use self::struct_::SerializeStruct;

mod map;
mod seq;
mod struct_;

/// Serialization result
pub type Result<T> = ::core::result::Result<T, Error>;

/// This type represents all possible errors that can occur when serializing JSON data
#[derive(Debug)]
pub enum Error {
    /// Buffer is full
    BufferFull,
    #[doc(hidden)]
    __Extensible,
}

impl From<()> for Error {
    fn from(_: ()) -> Error {
        Error::BufferFull
    }
}

impl From<u8> for Error {
    fn from(_: u8) -> Error {
        Error::BufferFull
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        ""
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer is full")
    }
}

pub(crate) struct Serializer<B>
where
    B: heapless::ArrayLength<u8>,
{
    buf: Vec<u8, B>,
}

impl<B> Serializer<B>
where
    B: heapless::ArrayLength<u8>,
{
    fn new() -> Self {
        Serializer { buf: Vec::new() }
    }
}

// NOTE(serialize_*signed) This is basically the numtoa implementation minus the lookup tables,
// which take 200+ bytes of ROM / Flash
macro_rules! serialize_unsigned {
    ($self:ident, $N:expr, $v:expr) => {{
        let mut buf: [u8; $N] = unsafe { super::uninitialized() };

        let mut v = $v;
        let mut i = $N - 1;
        loop {
            buf[i] = (v % 10) as u8 + b'0';
            v /= 10;

            if v == 0 {
                break;
            } else {
                i -= 1;
            }
        }

        $self.buf.extend_from_slice(&buf[i..])?;
        Ok(())
    }};
}

macro_rules! serialize_signed {
    ($self:ident, $N:expr, $v:expr, $ixx:ident, $uxx:ident) => {{
        let v = $v;
        let (signed, mut v) = if v == $ixx::min_value() {
            (true, $ixx::max_value() as $uxx + 1)
        } else if v < 0 {
            (true, -v as $uxx)
        } else {
            (false, v as $uxx)
        };

        let mut buf: [u8; $N] = unsafe { super::uninitialized() };
        let mut i = $N - 1;
        loop {
            buf[i] = (v % 10) as u8 + b'0';
            v /= 10;

            i -= 1;

            if v == 0 {
                break;
            }
        }

        if signed {
            buf[i] = b'-';
        } else {
            i += 1;
        }
        $self.buf.extend_from_slice(&buf[i..])?;
        Ok(())
    }};
}

macro_rules! serialize_fmt {
    ($self:ident, $uxx:ident, $fmt:expr, $v:expr) => {{
        let mut s: String<$uxx> = String::new();
        write!(&mut s, $fmt, $v).unwrap();
        $self.buf.extend_from_slice(s.as_bytes())?;
        Ok(())
    }};
}

impl<'a, B> ser::Serializer for &'a mut Serializer<B>
where
    B: heapless::ArrayLength<u8>,
{
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializeSeq<'a, B>;
    type SerializeTuple = SerializeSeq<'a, B>;
    type SerializeTupleStruct = Unreachable;
    type SerializeTupleVariant = Unreachable;
    type SerializeMap = SerializeMap<'a, B>;
    type SerializeStruct = SerializeStruct<'a, B>;
    type SerializeStructVariant = Unreachable;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        if v {
            self.buf.extend_from_slice(b"true")?;
        } else {
            self.buf.extend_from_slice(b"false")?;
        }

        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        // "-128"
        serialize_signed!(self, 4, v, i8, u8)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        // "-32768"
        serialize_signed!(self, 6, v, i16, u16)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        // "-2147483648"
        serialize_signed!(self, 11, v, i32, u32)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        // "-9223372036854775808"
        serialize_signed!(self, 20, v, i64, u64)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        // "255"
        serialize_unsigned!(self, 3, v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        // "65535"
        serialize_unsigned!(self, 5, v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        // "4294967295"
        serialize_unsigned!(self, 10, v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        // "18446744073709551615"
        serialize_unsigned!(self, 20, v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        serialize_fmt!(self, U16, "{:e}", v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        serialize_fmt!(self, U32, "{:e}", v)
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.buf.push(b'"')?;
        self.buf.extend_from_slice(v.as_bytes())?;
        self.buf.push(b'"')?;
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.buf.extend_from_slice(b"null")?;
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        unreachable!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        unreachable!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.buf.push(b'[')?;

        Ok(SerializeSeq::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(_len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        unreachable!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        unreachable!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.buf.push(b'{')?;

        Ok(SerializeMap::new(self))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.buf.push(b'{')?;

        Ok(SerializeStruct::new(self))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        unreachable!()
    }

    fn collect_str<T: ?Sized>(self, _value: &T) -> Result<Self::Ok>
    where
        T: fmt::Display,
    {
        unreachable!()
    }
}

/// Serializes the given data structure as a string of JSON text
pub fn to_string<B, T>(value: &T) -> Result<String<B>>
where
    B: heapless::ArrayLength<u8>,
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::new();
    value.serialize(&mut ser)?;
    Ok(unsafe { String::from_utf8_unchecked(ser.buf) })
}

/// Serializes the given data structure as a JSON byte vector
pub fn to_vec<B, T>(value: &T) -> Result<Vec<u8, B>>
where
    B: heapless::ArrayLength<u8>,
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::new();
    value.serialize(&mut ser)?;
    Ok(ser.buf)
}

impl ser::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: fmt::Display,
    {
        unreachable!()
    }
}

pub(crate) enum Unreachable {}

impl ser::SerializeTupleStruct for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<()> {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

impl ser::SerializeTupleVariant for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<()> {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

impl ser::SerializeMap for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        unreachable!()
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

impl ser::SerializeStructVariant for Unreachable {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: ser::Serialize,
    {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok> {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use serde_derive::Serialize;

    use heapless::consts::U128;

    type N = U128;

    #[test]
    fn array() {
        assert_eq!(&*crate::to_string::<N, _>(&[0, 1, 2]).unwrap(), "[0,1,2]");
    }

    #[test]
    fn bool() {
        assert_eq!(&*crate::to_string::<N, _>(&true).unwrap(), "true");
    }

    #[test]
    fn enum_() {
        #[derive(Serialize)]
        enum Type {
            #[serde(rename = "boolean")]
            Boolean,
            #[serde(rename = "number")]
            Number,
        }

        assert_eq!(
            &*crate::to_string::<N, _>(&Type::Boolean).unwrap(),
            r#""boolean""#
        );

        assert_eq!(
            &*crate::to_string::<N, _>(&Type::Number).unwrap(),
            r#""number""#
        );
    }

    #[test]
    fn str() {
        assert_eq!(&*crate::to_string::<N, _>("hello").unwrap(), r#""hello""#);
    }

    #[test]
    fn struct_bool() {
        #[derive(Serialize)]
        struct Led {
            led: bool,
        }

        assert_eq!(
            &*crate::to_string::<N, _>(&Led { led: true }).unwrap(),
            r#"{"led":true}"#
        );
    }

    #[test]
    fn struct_i8() {
        #[derive(Serialize)]
        struct Temperature {
            temperature: i8,
        }

        assert_eq!(
            &*crate::to_string::<N, _>(&Temperature { temperature: 127 }).unwrap(),
            r#"{"temperature":127}"#
        );

        assert_eq!(
            &*crate::to_string::<N, _>(&Temperature { temperature: 20 }).unwrap(),
            r#"{"temperature":20}"#
        );

        assert_eq!(
            &*crate::to_string::<N, _>(&Temperature { temperature: -17 }).unwrap(),
            r#"{"temperature":-17}"#
        );

        assert_eq!(
            &*crate::to_string::<N, _>(&Temperature { temperature: -128 }).unwrap(),
            r#"{"temperature":-128}"#
        );
    }

    #[test]
    fn struct_f32() {
        #[derive(Serialize)]
        struct Temperature {
            temperature: f32,
        }

        assert_eq!(
            &*crate::to_string::<N, _>(&Temperature { temperature: -20. }).unwrap(),
            r#"{"temperature":-2e1}"#
        );

        assert_eq!(
            &*crate::to_string::<N, _>(&Temperature {
                temperature: -20345.
            })
            .unwrap(),
            r#"{"temperature":-2.0345e4}"#
        );

        assert_eq!(
            &*crate::to_string::<N, _>(&Temperature {
                temperature: -2.3456789012345e-23
            })
            .unwrap(),
            r#"{"temperature":-2.3456788e-23}"#
        );
    }

    #[test]
    fn struct_option() {
        #[derive(Serialize)]
        struct Property<'a> {
            description: Option<&'a str>,
        }

        assert_eq!(
            crate::to_string::<N, _>(&Property {
                description: Some("An ambient temperature sensor"),
            })
            .unwrap(),
            r#"{"description":"An ambient temperature sensor"}"#
        );

        // XXX Ideally this should produce "{}"
        assert_eq!(
            crate::to_string::<N, _>(&Property { description: None }).unwrap(),
            r#"{"description":null}"#
        );
    }

    #[test]
    fn struct_u8() {
        #[derive(Serialize)]
        struct Temperature {
            temperature: u8,
        }

        assert_eq!(
            &*crate::to_string::<N, _>(&Temperature { temperature: 20 }).unwrap(),
            r#"{"temperature":20}"#
        );
    }

    #[test]
    fn struct_() {
        #[derive(Serialize)]
        struct Empty {}

        assert_eq!(&*crate::to_string::<N, _>(&Empty {}).unwrap(), r#"{}"#);

        #[derive(Serialize)]
        struct Tuple {
            a: bool,
            b: bool,
        }

        assert_eq!(
            &*crate::to_string::<N, _>(&Tuple { a: true, b: false }).unwrap(),
            r#"{"a":true,"b":false}"#
        );
    }
}
