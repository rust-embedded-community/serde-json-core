//! Serialize a Rust data structure into JSON data

use core::{
    fmt,
    mem::{self, MaybeUninit},
    str,
};

use serde::ser;
use serde::ser::SerializeStruct as _;

#[cfg(feature = "heapless")]
use heapless::{String, Vec};

use self::map::SerializeMap;
use self::seq::SerializeSeq;
use self::struct_::{SerializeStruct, SerializeStructVariant};

mod map;
mod seq;
mod struct_;

/// Serialization result
pub type Result<T> = ::core::result::Result<T, Error>;

/// This type represents all possible errors that can occur when serializing JSON data
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Buffer is full
    BufferFull,
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

impl serde::ser::StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer is full")
    }
}

pub(crate) struct Serializer<'a> {
    buf: &'a mut [u8],
    current_length: usize,
}

impl<'a> Serializer<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Serializer {
            buf,
            current_length: 0,
        }
    }

    fn push(&mut self, c: u8) -> Result<()> {
        if self.current_length < self.buf.len() {
            unsafe { self.push_unchecked(c) };
            Ok(())
        } else {
            Err(Error::BufferFull)
        }
    }

    unsafe fn push_unchecked(&mut self, c: u8) {
        self.buf[self.current_length] = c;
        self.current_length += 1;
    }

    fn extend_from_slice(&mut self, other: &[u8]) -> Result<()> {
        if self.current_length + other.len() > self.buf.len() {
            // won't fit in the buf; don't modify anything and return an error
            Err(Error::BufferFull)
        } else {
            for c in other {
                unsafe { self.push_unchecked(*c) };
            }
            Ok(())
        }
    }
}

// NOTE(serialize_*signed) This is basically the numtoa implementation minus the lookup tables,
// which take 200+ bytes of ROM / Flash
macro_rules! serialize_unsigned {
    ($self:ident, $N:expr, $v:expr) => {{
        let mut i = $N - 1;
        let mut v = $v;
        let buf = {
            let mut buf: [MaybeUninit<u8>; $N] = unsafe { MaybeUninit::uninit().assume_init() };
            loop {
                buf[i] = MaybeUninit::new((v % 10) as u8 + b'0');
                v /= 10;

                if v == 0 {
                    break;
                } else {
                    i -= 1;
                }
            }
            unsafe { mem::transmute::<_, [u8; $N]>(buf) }
        };

        $self.extend_from_slice(&buf[i..])
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

        let mut i = $N - 1;
        let mut buf = {
            let mut buf: [MaybeUninit<u8>; $N] = unsafe { MaybeUninit::uninit().assume_init() };
            loop {
                buf[i] = MaybeUninit::new((v % 10) as u8 + b'0');
                v /= 10;

                i -= 1;

                if v == 0 {
                    break;
                }
            }
            unsafe { mem::transmute::<_, [u8; $N]>(buf) }
        };

        if signed {
            buf[i] = b'-';
        } else {
            i += 1;
        }
        $self.extend_from_slice(&buf[i..])
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

impl<'a, 'b: 'a> ser::Serializer for &'a mut Serializer<'b> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializeSeq<'a, 'b>;
    type SerializeTuple = SerializeSeq<'a, 'b>;
    type SerializeTupleStruct = Unreachable;
    type SerializeTupleVariant = Unreachable;
    type SerializeMap = SerializeMap<'a, 'b>;
    type SerializeStruct = SerializeStruct<'a, 'b>;
    type SerializeStructVariant = SerializeStructVariant<'a, 'b>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        if v {
            self.extend_from_slice(b"true")
        } else {
            self.extend_from_slice(b"false")
        }
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
        serialize_ryu!(self, v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        serialize_ryu!(self, v)
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.push(b'"')?;

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

        for c in v.chars() {
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
        }

        self.push(b'"')
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.extend_from_slice(v)
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.extend_from_slice(b"null")
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ser::Serialize,
    {
        self.push(b'{')?;
        let mut s = SerializeStruct::new(&mut self);
        s.serialize_field(variant, value)?;
        s.end()?;
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.push(b'[')?;

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
        self.push(b'{')?;

        Ok(SerializeMap::new(self))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.push(b'{')?;

        Ok(SerializeStruct::new(self))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.extend_from_slice(b"{\"")?;
        self.extend_from_slice(variant.as_bytes())?;
        self.extend_from_slice(b"\":{")?;

        Ok(SerializeStructVariant::new(self))
    }

    fn collect_str<T: ?Sized>(self, _value: &T) -> Result<Self::Ok>
    where
        T: fmt::Display,
    {
        unreachable!()
    }
}

/// Serializes the given data structure as a string of JSON text
#[cfg(feature = "heapless")]
pub fn to_string<T, const N: usize>(value: &T) -> Result<String<N>>
where
    T: ser::Serialize + ?Sized,
{
    Ok(unsafe { str::from_utf8_unchecked(&to_vec::<T, N>(value)?) }.into())
}

/// Serializes the given data structure as a JSON byte vector
#[cfg(feature = "heapless")]
pub fn to_vec<T, const N: usize>(value: &T) -> Result<Vec<u8, N>>
where
    T: ser::Serialize + ?Sized,
{
    let mut buf = Vec::<u8, N>::new();
    buf.resize_default(N)?;
    let len = to_slice(value, &mut buf)?;
    buf.truncate(len);
    Ok(buf)
}

/// Serializes the given data structure as a JSON byte vector into the provided buffer
pub fn to_slice<T>(value: &T, buf: &mut [u8]) -> Result<usize>
where
    T: ser::Serialize + ?Sized,
{
    let mut ser = Serializer::new(buf);
    value.serialize(&mut ser)?;
    Ok(ser.current_length)
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
            &*crate::to_string::<_, N>(" \u{001f} ").unwrap(),
            r#"" \u001F ""#
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
                serializer.serialize_bytes(&aux.as_bytes())
            }
        }

        let sd1 = SimpleDecimal(1.55555);
        assert_eq!(&*crate::to_string::<_, N>(&sd1).unwrap(), r#"1.56"#);

        let sd2 = SimpleDecimal(0.000);
        assert_eq!(&*crate::to_string::<_, N>(&sd2).unwrap(), r#"0.00"#);

        let sd3 = SimpleDecimal(22222.777777);
        assert_eq!(&*crate::to_string::<_, N>(&sd3).unwrap(), r#"22222.78"#);
    }
}
