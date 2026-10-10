//! Serialize a Rust data structure into JSON data

use core::fmt::Display;
use core::mem::MaybeUninit;
use core::{fmt, str};

#[cfg(feature = "heapless")]
use embedded_io::SliceWriteError;
use embedded_io::Write;

use serde::ser;
use serde::ser::SerializeStruct as _;
use serde::Serialize;

#[cfg(feature = "heapless")]
use heapless::{String, Vec};

mod map;
use map::SerializeMap;

mod seq;
use seq::SerializeSeq;

mod struct_;
use struct_::{SerializeStruct, SerializeStructVariant};

mod error;
pub use error::Error;

mod unreachable;
use unreachable::Unreachable;

/// A structure that serializes Rust values as JSON into a buffer.
pub struct Serializer<W> {
    writer: W,
}

impl<W: Write> Serializer<W> {
    /// Create a new `Serializer`
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Consume the `Serializer` and return the underlying writer
    pub fn into_inner(self) -> W {
        self.writer
    }

    fn push(&mut self, c: u8) -> Result<(), Error<W::Error>> {
        Ok(self.writer.write_all(&[c])?)
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), Error<W::Error>> {
        Ok(self.writer.write_all(other)?)
    }

    fn push_char(&mut self, c: char) -> Result<(), Error<W::Error>> {
        // Do escaping according to "6. MUST represent all strings (including object member names) in
        // their minimal-length UTF-8 encoding": https://gibson042.github.io/canonicaljson-spec/
        //
        // We don't need to escape lone surrogates because surrogate pairs do not exist in valid UTF-8,
        // even if they can exist in JSON or JavaScript strings (UCS-2 based). As a result, lone surrogates
        // cannot exist in a Rust String. If they do, the bug is in the String constructor.
        // An excellent explanation is available at https://www.youtube.com/watch?v=HhIEDWmQS3w

        // Temporary storage for encoded a single char.
        // A char is up to 4 bytes long wehn encoded to UTF-8.
        let mut encoding_tmp = [0u8; 4];

        match c {
            '\\' => {
                self.push(b'\\')?;
                self.push(b'\\')?;
            }
            '"' => {
                self.push(b'\\')?;
                self.push(b'"')?;
            }
            '\u{0008}' => {
                self.push(b'\\')?;
                self.push(b'b')?;
            }
            '\u{0009}' => {
                self.push(b'\\')?;
                self.push(b't')?;
            }
            '\u{000A}' => {
                self.push(b'\\')?;
                self.push(b'n')?;
            }
            '\u{000C}' => {
                self.push(b'\\')?;
                self.push(b'f')?;
            }
            '\u{000D}' => {
                self.push(b'\\')?;
                self.push(b'r')?;
            }
            '\u{0000}'..='\u{001F}' => {
                self.push(b'\\')?;
                self.push(b'u')?;
                self.push(b'0')?;
                self.push(b'0')?;
                let (hex1, hex2) = hex(c as u8);
                self.push(hex1)?;
                self.push(hex2)?;
            }
            _ => {
                let encoded = c.encode_utf8(&mut encoding_tmp as &mut [u8]);
                self.extend_from_slice(encoded.as_bytes())?;
            }
        }

        Ok(())
    }
}

// NOTE(serialize_*signed) This is basically the numtoa implementation minus the lookup tables,
// which take 200+ bytes of ROM / Flash
macro_rules! serialize_unsigned {
    ($self:ident, $N:expr, $v:expr) => {{
        let mut buf: [MaybeUninit<u8>; $N] = [MaybeUninit::uninit(); $N];

        let mut v = $v;
        let mut i = $N - 1;
        loop {
            buf[i].write((v % 10) as u8 + b'0');
            v /= 10;

            if v == 0 {
                break;
            } else {
                i -= 1;
            }
        }

        // Note(feature): maybe_uninit_slice
        let buf = unsafe { &*(&buf[i..] as *const _ as *const [u8]) };

        $self.extend_from_slice(buf)
    }};
}

macro_rules! serialize_signed {
    ($self:ident, $N:expr, $v:expr, $ixx:ident, $uxx:ident) => {{
        let v = $v;
        let (signed, mut v) = if v == $ixx::MIN {
            (true, $ixx::MAX as $uxx + 1)
        } else if v < 0 {
            (true, -v as $uxx)
        } else {
            (false, v as $uxx)
        };

        let mut buf: [MaybeUninit<u8>; $N] = [MaybeUninit::uninit(); $N];
        let mut i = $N - 1;
        loop {
            buf[i].write((v % 10) as u8 + b'0');
            v /= 10;

            i -= 1;

            if v == 0 {
                break;
            }
        }

        if signed {
            buf[i].write(b'-');
        } else {
            i += 1;
        }

        // Note(feature): maybe_uninit_slice
        let buf = unsafe { &*(&buf[i..] as *const _ as *const [u8]) };

        $self.extend_from_slice(buf)
    }};
}

macro_rules! serialize_ryu {
    ($self:ident, $v:expr) => {{
        let mut buffer = ryu::Buffer::new();
        let printed = buffer.format($v);
        $self.extend_from_slice(printed.as_bytes())
    }};
}

/// Upper-case hex for value in 0..16, encoded as ASCII bytes
fn hex_4bit(c: u8) -> u8 {
    if c <= 9 {
        0x30 + c
    } else {
        0x41 + (c - 10)
    }
}

/// Upper-case hex for value in 0..256, encoded as ASCII bytes
fn hex(c: u8) -> (u8, u8) {
    (hex_4bit(c >> 4), hex_4bit(c & 0x0F))
}

impl<'a, W: Write<Error: Display>> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error<W::Error>;
    type SerializeSeq = SerializeSeq<'a, W>;
    type SerializeTuple = SerializeSeq<'a, W>;
    type SerializeTupleStruct = SerializeSeq<'a, W>;
    type SerializeTupleVariant = Unreachable<W::Error>;
    type SerializeMap = SerializeMap<'a, W>;
    type SerializeStruct = SerializeStruct<'a, W>;
    type SerializeStructVariant = SerializeStructVariant<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        if v {
            self.extend_from_slice(b"true")
        } else {
            self.extend_from_slice(b"false")
        }
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        // "-128"
        serialize_signed!(self, 4, v, i8, u8)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        // "-32768"
        serialize_signed!(self, 6, v, i16, u16)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        // "-2147483648"
        serialize_signed!(self, 11, v, i32, u32)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        // "-9223372036854775808"
        serialize_signed!(self, 20, v, i64, u64)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        // "255"
        serialize_unsigned!(self, 3, v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        // "65535"
        serialize_unsigned!(self, 5, v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        // "4294967295"
        serialize_unsigned!(self, 10, v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        // "18446744073709551615"
        serialize_unsigned!(self, 20, v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        if v.is_finite() {
            serialize_ryu!(self, v)
        } else {
            self.serialize_none()
        }
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        if v.is_finite() {
            serialize_ryu!(self, v)
        } else {
            self.serialize_none()
        }
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        unreachable!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.push(b'"')?;

        for c in v.chars() {
            self.push_char(c)?;
        }

        self.push(b'"')
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.extend_from_slice(v)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.extend_from_slice(b"null")
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        // If the newtype struct is an `EscapedStr`...
        if name == crate::str::EscapedStr::NAME {
            // serialize it as an already escaped string.

            struct EscapedStringSerializer<'a, W>(&'a mut Serializer<W>);

            impl<'a, W: Write<Error: Display>> serde::Serializer for EscapedStringSerializer<'a, W> {
                type Ok = ();
                type Error = Error<W::Error>;

                type SerializeSeq = serde::ser::Impossible<(), Self::Error>;
                type SerializeTuple = serde::ser::Impossible<(), Self::Error>;
                type SerializeTupleStruct = serde::ser::Impossible<(), Self::Error>;
                type SerializeTupleVariant = serde::ser::Impossible<(), Self::Error>;
                type SerializeMap = serde::ser::Impossible<(), Self::Error>;
                type SerializeStruct = serde::ser::Impossible<(), Self::Error>;
                type SerializeStructVariant = serde::ser::Impossible<(), Self::Error>;

                fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
                    v.bytes().try_for_each(|c| self.0.push(c))
                }

                fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_some<T: Serialize + ?Sized>(
                    self,
                    _value: &T,
                ) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_unit_struct(
                    self,
                    _name: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_unit_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_newtype_struct<T: Serialize + ?Sized>(
                    self,
                    _name: &'static str,
                    _value: &T,
                ) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_newtype_variant<T: Serialize + ?Sized>(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _value: &T,
                ) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }

                fn serialize_seq(
                    self,
                    _len: Option<usize>,
                ) -> Result<Self::SerializeSeq, Self::Error> {
                    unreachable!()
                }

                fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
                    unreachable!()
                }

                fn serialize_tuple_struct(
                    self,
                    _name: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                    unreachable!()
                }

                fn serialize_tuple_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                    unreachable!()
                }

                fn serialize_map(
                    self,
                    _len: Option<usize>,
                ) -> Result<Self::SerializeMap, Self::Error> {
                    unreachable!()
                }

                fn serialize_struct(
                    self,
                    _name: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeStruct, Self::Error> {
                    unreachable!()
                }

                fn serialize_struct_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeStructVariant, Self::Error> {
                    unreachable!()
                }

                fn collect_str<T: fmt::Display + ?Sized>(
                    self,
                    _value: &T,
                ) -> Result<Self::Ok, Self::Error> {
                    unreachable!()
                }
            }

            self.push(b'"')?;

            value.serialize(EscapedStringSerializer(self))?;

            self.push(b'"')?;

            Ok(())
        } else {
            value.serialize(self)
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ser::Serialize + ?Sized,
    {
        self.push(b'{')?;
        let mut s = SerializeStruct::new(self);
        s.serialize_field(variant, value)?;
        s.end()?;
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.push(b'[')?;

        Ok(SerializeSeq::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(_len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unreachable!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.push(b'{')?;

        Ok(SerializeMap::new(self))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.push(b'{')?;

        Ok(SerializeStruct::new(self))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.extend_from_slice(b"{\"")?;
        self.extend_from_slice(variant.as_bytes())?;
        self.extend_from_slice(b"\":{")?;

        Ok(SerializeStructVariant::new(self))
    }

    fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: fmt::Display + ?Sized,
    {
        self.push(b'"')?;
        self.writer.write_fmt(format_args!("{}", value))?;
        self.push(b'"')
    }
}

/// Serializes the given data structure as a string of JSON text
#[cfg(feature = "heapless")]
pub fn to_string<T, const N: usize>(value: &T) -> Result<String<N>, Error<SliceWriteError>>
where
    T: ser::Serialize + ?Sized,
{
    Ok(unsafe { String::from_utf8_unchecked(to_vec::<T, N>(value)?) })
}

/// Serializes the given data structure as a JSON byte vector
#[cfg(feature = "heapless")]
pub fn to_vec<T, const N: usize>(value: &T) -> Result<Vec<u8, N>, Error<SliceWriteError>>
where
    T: ser::Serialize + ?Sized,
{
    let mut buf = Vec::<u8, N>::new();
    buf.resize_default(N).map_err(|_| SliceWriteError::Full)?;
    let len = to_slice(value, &mut buf)?;
    buf.truncate(len);
    Ok(buf)
}

/// Serializes the given data structure as a JSON byte vector into the provided buffer
pub fn to_slice<T>(value: &T, buf: &mut [u8]) -> Result<usize, Error<SliceWriteError>>
where
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::new(&mut *buf);
    value.serialize(&mut ser)?;
    let rem = ser.into_inner().len();
    Ok(buf.len() - rem)
}

#[cfg(test)]
mod tests {
    use serde_derive::Serialize;

    const N: usize = 128;

    #[test]
    fn array() {
        let buf = &mut [0u8; 128];
        let len = crate::to_slice(&[0, 1, 2], buf).unwrap();
        assert_eq!(len, 7);
        assert_eq!(&buf[..len], b"[0,1,2]");
        assert_eq!(&*crate::to_string::<_, N>(&[0, 1, 2]).unwrap(), "[0,1,2]");
    }

    #[test]
    fn bool() {
        let buf = &mut [0u8; 128];
        let len = crate::to_slice(&true, buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], b"true");

        assert_eq!(&*crate::to_string::<_, N>(&true).unwrap(), "true");
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
            &*crate::to_string::<_, N>(&Type::Boolean).unwrap(),
            r#""boolean""#
        );

        assert_eq!(
            &*crate::to_string::<_, N>(&Type::Number).unwrap(),
            r#""number""#
        );
    }

    #[test]
    fn str() {
        assert_eq!(&*crate::to_string::<_, N>("hello").unwrap(), r#""hello""#);
        assert_eq!(&*crate::to_string::<_, N>("").unwrap(), r#""""#);

        // Characters unescaped if possible
        assert_eq!(&*crate::to_string::<_, N>("ä").unwrap(), r#""ä""#);
        assert_eq!(&*crate::to_string::<_, N>("৬").unwrap(), r#""৬""#);
        // assert_eq!(&*crate::to_string::<_, N>("\u{A0}").unwrap(), r#"" ""#); // non-breaking space
        assert_eq!(&*crate::to_string::<_, N>("ℝ").unwrap(), r#""ℝ""#); // 3 byte character
        assert_eq!(&*crate::to_string::<_, N>("💣").unwrap(), r#""💣""#); // 4 byte character

        // " and \ must be escaped
        assert_eq!(
            &*crate::to_string::<_, N>("foo\"bar").unwrap(),
            r#""foo\"bar""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>("foo\\bar").unwrap(),
            r#""foo\\bar""#
        );

        // \b, \t, \n, \f, \r must be escaped in their two-character escaping
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{0008} ").unwrap(),
            r#"" \b ""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{0009} ").unwrap(),
            r#"" \t ""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{000A} ").unwrap(),
            r#"" \n ""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{000C} ").unwrap(),
            r#"" \f ""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{000D} ").unwrap(),
            r#"" \r ""#
        );

        // U+0000 through U+001F is escaped using six-character \u00xx uppercase hexadecimal escape sequences
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{0000} ").unwrap(),
            r#"" \u0000 ""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{0001} ").unwrap(),
            r#"" \u0001 ""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{0007} ").unwrap(),
            r#"" \u0007 ""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{000e} ").unwrap(),
            r#"" \u000E ""#
        );
        assert_eq!(
            &*crate::to_string::<_, N>(" \u{001D} ").unwrap(),
            r#"" \u001D ""#
        );
        assert_eq!(
            crate::to_string::<_, N>(" \u{001f} ").unwrap(),
            r#"" \u001F ""#
        );
    }

    #[test]
    fn escaped_str() {
        assert_eq!(
            crate::to_string::<_, N>(&crate::str::EscapedStr(r#"Hello\\nWorld"#)).unwrap(),
            r#""Hello\\nWorld""#
        );
    }

    #[test]
    fn struct_bool() {
        #[derive(Serialize)]
        struct Led {
            led: bool,
        }

        assert_eq!(
            &*crate::to_string::<_, N>(&Led { led: true }).unwrap(),
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
            &*crate::to_string::<_, N>(&Temperature { temperature: 127 }).unwrap(),
            r#"{"temperature":127}"#
        );

        assert_eq!(
            &*crate::to_string::<_, N>(&Temperature { temperature: 20 }).unwrap(),
            r#"{"temperature":20}"#
        );

        assert_eq!(
            &*crate::to_string::<_, N>(&Temperature { temperature: -17 }).unwrap(),
            r#"{"temperature":-17}"#
        );

        assert_eq!(
            &*crate::to_string::<_, N>(&Temperature { temperature: -128 }).unwrap(),
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
            &*crate::to_string::<_, N>(&Temperature { temperature: -20. }).unwrap(),
            r#"{"temperature":-20.0}"#
        );

        assert_eq!(
            &*crate::to_string::<_, N>(&Temperature {
                temperature: -20345.
            })
            .unwrap(),
            r#"{"temperature":-20345.0}"#
        );

        assert_eq!(
            &*crate::to_string::<_, N>(&Temperature {
                temperature: -2.345_678_8e-23
            })
            .unwrap(),
            r#"{"temperature":-2.3456788e-23}"#
        );

        assert_eq!(
            &*crate::to_string::<_, N>(&Temperature {
                temperature: f32::NAN
            })
            .unwrap(),
            r#"{"temperature":null}"#
        );

        assert_eq!(
            &*crate::to_string::<_, N>(&Temperature {
                temperature: f32::NEG_INFINITY
            })
            .unwrap(),
            r#"{"temperature":null}"#
        );
    }

    #[test]
    fn struct_option() {
        #[derive(Serialize)]
        struct Property<'a> {
            description: Option<&'a str>,
        }

        assert_eq!(
            crate::to_string::<_, N>(&Property {
                description: Some("An ambient temperature sensor"),
            })
            .unwrap(),
            r#"{"description":"An ambient temperature sensor"}"#
        );

        // XXX Ideally this should produce "{}"
        assert_eq!(
            crate::to_string::<_, N>(&Property { description: None }).unwrap(),
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
            &*crate::to_string::<_, N>(&Temperature { temperature: 20 }).unwrap(),
            r#"{"temperature":20}"#
        );
    }

    #[test]
    fn struct_() {
        #[derive(Serialize)]
        struct Empty {}

        assert_eq!(&*crate::to_string::<_, N>(&Empty {}).unwrap(), r#"{}"#);

        #[derive(Serialize)]
        struct Tuple {
            a: bool,
            b: bool,
        }

        assert_eq!(
            &*crate::to_string::<_, N>(&Tuple { a: true, b: false }).unwrap(),
            r#"{"a":true,"b":false}"#
        );
    }

    #[test]
    fn test_unit() {
        let a = ();
        assert_eq!(&*crate::to_string::<_, N>(&a).unwrap(), r#"null"#);
    }

    #[test]
    fn test_newtype_struct() {
        #[derive(Serialize)]
        struct A(pub u32);
        let a = A(54);
        assert_eq!(&*crate::to_string::<_, N>(&a).unwrap(), r#"54"#);
    }

    #[test]
    fn test_newtype_variant() {
        #[derive(Serialize)]
        enum A {
            A(u32),
        }
        let a = A::A(54);

        assert_eq!(&*crate::to_string::<_, N>(&a).unwrap(), r#"{"A":54}"#);
    }

    #[test]
    fn test_struct_variant() {
        #[derive(Serialize)]
        enum A {
            A { x: u32, y: u16 },
        }
        let a = A::A { x: 54, y: 720 };

        assert_eq!(
            &*crate::to_string::<_, N>(&a).unwrap(),
            r#"{"A":{"x":54,"y":720}}"#
        );
    }

    #[test]
    fn test_tuple_struct() {
        #[derive(Serialize)]
        struct A<'a>(u32, Option<&'a str>, u16, bool);

        let a = A(42, Some("A string"), 720, false);

        assert_eq!(
            &*crate::to_string::<_, N>(&a).unwrap(),
            r#"[42,"A string",720,false]"#
        );
    }

    #[test]
    fn test_tuple_struct_roundtrip() {
        use serde_derive::Deserialize;

        #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
        struct A<'a>(u32, Option<&'a str>, u16, bool);

        let a1 = A(42, Some("A string"), 720, false);
        let serialized = crate::to_string::<_, N>(&a1).unwrap();
        let (a2, _size): (A<'_>, usize) = crate::from_str(&serialized).unwrap();
        assert_eq!(a1, a2);
    }

    #[test]
    fn test_serialize_bytes() {
        use core::fmt::Write;
        use heapless::String;

        pub struct SimpleDecimal(f32);

        impl serde::Serialize for SimpleDecimal {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut aux: String<{ N }> = String::new();
                write!(aux, "{:.2}", self.0).unwrap();
                serializer.serialize_bytes(aux.as_bytes())
            }
        }

        let sd1 = SimpleDecimal(1.55555);
        assert_eq!(&*crate::to_string::<_, N>(&sd1).unwrap(), r#"1.56"#);

        let sd2 = SimpleDecimal(0.000);
        assert_eq!(&*crate::to_string::<_, N>(&sd2).unwrap(), r#"0.00"#);

        let sd3 = SimpleDecimal(22_222.777);
        assert_eq!(&*crate::to_string::<_, N>(&sd3).unwrap(), r#"22222.78"#);
    }
}
