//! Deserialize JSON data to a Rust data structure

use core::str::FromStr;
use core::{fmt, str};

use serde::de::{self, Visitor};

use self::enum_::{UnitVariantAccess, VariantAccess};
use self::map::MapAccess;
use self::seq::SeqAccess;

mod enum_;
mod map;
mod seq;

/// Deserialization result
pub type Result<T> = core::result::Result<T, Error>;

/// This type represents all possible errors that can occur when deserializing JSON data
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// EOF while parsing a list.
    EofWhileParsingList,

    /// EOF while parsing an object.
    EofWhileParsingObject,

    /// EOF while parsing a string.
    EofWhileParsingString,

    /// EOF while parsing a JSON number.
    EofWhileParsingNumber,

    /// EOF while parsing a JSON value.
    EofWhileParsingValue,

    /// Expected this character to be a `':'`.
    ExpectedColon,

    /// Expected this character to be either a `','` or a `']'`.
    ExpectedListCommaOrEnd,

    /// Expected this character to be either a `','` or a `'}'`.
    ExpectedObjectCommaOrEnd,

    /// Expected to parse either a `true`, `false`, or a `null`.
    ExpectedSomeIdent,

    /// Expected this character to start a JSON value.
    ExpectedSomeValue,

    /// Invalid number.
    InvalidNumber,

    /// Invalid type
    InvalidType,

    /// Invalid unicode code point.
    InvalidUnicodeCodePoint,

    /// Object key is not a string.
    KeyMustBeAString,

    /// JSON has non-whitespace trailing characters after the value.
    TrailingCharacters,

    /// JSON has a comma after the last value in an array or map.
    TrailingComma,

    /// Error with a custom message that we had to discard.
    CustomError,

    /// Error with a custom message that was preserved.
    #[cfg(feature = "custom-error-messages")]
    CustomErrorWithMessage(heapless::String<heapless::consts::U64>),
}

impl serde::de::StdError for Error {}

pub(crate) struct Deserializer<'b> {
    slice: &'b [u8],
    index: usize,
}

impl<'a> Deserializer<'a> {
    fn new(slice: &'a [u8]) -> Deserializer<'_> {
        Deserializer { slice, index: 0 }
    }

    fn eat_char(&mut self) {
        self.index += 1;
    }

    fn end(&mut self) -> Result<usize> {
        match self.parse_whitespace() {
            Some(_) => Err(Error::TrailingCharacters),
            None => Ok(self.index),
        }
    }

    fn end_seq(&mut self) -> Result<()> {
        match self.parse_whitespace().ok_or(Error::EofWhileParsingList)? {
            b']' => {
                self.eat_char();
                Ok(())
            }
            b',' => {
                self.eat_char();
                match self.parse_whitespace() {
                    Some(b']') => Err(Error::TrailingComma),
                    _ => Err(Error::TrailingCharacters),
                }
            }
            _ => Err(Error::TrailingCharacters),
        }
    }

    fn end_map(&mut self) -> Result<()> {
        match self
            .parse_whitespace()
            .ok_or(Error::EofWhileParsingObject)?
        {
            b'}' => {
                self.eat_char();
                Ok(())
            }
            b',' => Err(Error::TrailingComma),
            _ => Err(Error::TrailingCharacters),
        }
    }

    fn next_char(&mut self) -> Option<u8> {
        let ch = self.slice.get(self.index);

        if ch.is_some() {
            self.index += 1;
        }

        ch.cloned()
    }

    fn parse_ident(&mut self, ident: &[u8]) -> Result<()> {
        for c in ident {
            if Some(*c) != self.next_char() {
                return Err(Error::ExpectedSomeIdent);
            }
        }

        Ok(())
    }

    fn parse_object_colon(&mut self) -> Result<()> {
        match self
            .parse_whitespace()
            .ok_or(Error::EofWhileParsingObject)?
        {
            b':' => {
                self.eat_char();
                Ok(())
            }
            _ => Err(Error::ExpectedColon),
        }
    }

    fn parse_str(&mut self) -> Result<&'a str> {
        let start = self.index;
        loop {
            match self.peek() {
                Some(b'"') => {
                    // Counts the number of backslashes in front of the current index.
                    //
                    // "some string with \\\" included."
                    //                  ^^^^^
                    //                  |||||
                    //       loop run:  4321|
                    //                      |
                    //                   `index`
                    //
                    // Since we only get in this code branch if we found a " starting the string and `index` is greater
                    // than the start position, we know the loop will end no later than this point.
                    let leading_backslashes = |index: usize| -> usize {
                        let mut count = 0;
                        loop {
                            if self.slice[index - count - 1] == b'\\' {
                                count += 1;
                            } else {
                                return count;
                            }
                        }
                    };

                    let is_escaped = leading_backslashes(self.index) % 2 == 1;
                    if is_escaped {
                        self.eat_char(); // just continue
                    } else {
                        let end = self.index;
                        self.eat_char();
                        return str::from_utf8(&self.slice[start..end])
                            .map_err(|_| Error::InvalidUnicodeCodePoint);
                    }
                }
                Some(_) => self.eat_char(),
                None => return Err(Error::EofWhileParsingString),
            }
        }
    }

    /// Consumes all the whitespace characters and returns a peek into the next character
    fn parse_whitespace(&mut self) -> Option<u8> {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\n') | Some(b'\t') | Some(b'\r') => {
                    self.eat_char();
                }
                other => {
                    return other;
                }
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.slice.get(self.index).cloned()
    }
}

// NOTE(deserialize_*signed) we avoid parsing into u64 and then casting to a smaller integer, which
// is what upstream does, to avoid pulling in 64-bit compiler intrinsics, which waste a few KBs of
// Flash, when targeting non 64-bit architectures
macro_rules! deserialize_unsigned {
    ($self:ident, $visitor:ident, $uxx:ident, $visit_uxx:ident) => {{
        let peek = $self
            .parse_whitespace()
            .ok_or(Error::EofWhileParsingValue)?;

        match peek {
            b'-' => Err(Error::InvalidNumber),
            b'0' => {
                $self.eat_char();
                $visitor.$visit_uxx(0)
            }
            b'1'..=b'9' => {
                $self.eat_char();

                let mut number = (peek - b'0') as $uxx;
                loop {
                    match $self.peek() {
                        Some(c @ b'0'..=b'9') => {
                            $self.eat_char();
                            number = number
                                .checked_mul(10)
                                .ok_or(Error::InvalidNumber)?
                                .checked_add((c - b'0') as $uxx)
                                .ok_or(Error::InvalidNumber)?;
                        }
                        _ => return $visitor.$visit_uxx(number),
                    }
                }
            }
            _ => Err(Error::InvalidType),
        }
    }};
}

macro_rules! deserialize_signed {
    ($self:ident, $visitor:ident, $ixx:ident, $visit_ixx:ident) => {{
        let signed = match $self
            .parse_whitespace()
            .ok_or(Error::EofWhileParsingValue)?
        {
            b'-' => {
                $self.eat_char();
                true
            }
            _ => false,
        };

        match $self.peek().ok_or(Error::EofWhileParsingValue)? {
            b'0' => {
                $self.eat_char();
                $visitor.$visit_ixx(0)
            }
            c @ b'1'..=b'9' => {
                $self.eat_char();

                let mut number = (c - b'0') as $ixx * if signed { -1 } else { 1 };
                loop {
                    match $self.peek() {
                        Some(c @ b'0'..=b'9') => {
                            $self.eat_char();
                            number = number
                                .checked_mul(10)
                                .ok_or(Error::InvalidNumber)?
                                .checked_add((c - b'0') as $ixx * if signed { -1 } else { 1 })
                                .ok_or(Error::InvalidNumber)?;
                        }
                        _ => return $visitor.$visit_ixx(number),
                    }
                }
            }
            _ => return Err(Error::InvalidType),
        }
    }};
}

macro_rules! deserialize_fromstr {
    ($self:ident, $visitor:ident, $typ:ident, $visit_fn:ident, $pattern:expr) => {{
        let start = $self.index;
        while $self.peek().is_some() {
            let c = $self.peek().unwrap();
            if $pattern.iter().find(|&&d| d == c).is_some() {
                $self.eat_char();
            } else {
                break;
            }
        }

        // Note(unsafe): We already checked that it only contains ascii. This is only true if the
        // caller has guaranteed that `pattern` contains only ascii characters.
        let s = unsafe { str::from_utf8_unchecked(&$self.slice[start..$self.index]) };

        let v = $typ::from_str(s).or(Err(Error::InvalidNumber))?;

        $visitor.$visit_fn(v)
    }};
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    /// Unsupported. Can’t parse a value without knowing its expected type.
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek = self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;

        match peek {
            b't' => {
                self.eat_char();
                self.parse_ident(b"rue")?;
                visitor.visit_bool(true)
            }
            b'f' => {
                self.eat_char();
                self.parse_ident(b"alse")?;
                visitor.visit_bool(false)
            }
            _ => Err(Error::InvalidType),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i8, visit_i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i16, visit_i16)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i32, visit_i32)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_signed!(self, visitor, i64, visit_i64)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u8, visit_u8)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u16, visit_u16)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u32, visit_u32)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        deserialize_unsigned!(self, visitor, u64, visit_u64)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
        deserialize_fromstr!(self, visitor, f32, visit_f32, b"0123456789+-.eE")
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;
        deserialize_fromstr!(self, visitor, f64, visit_f64, b"0123456789+-.eE")
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek = self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;

        match peek {
            b'"' => {
                self.eat_char();
                visitor.visit_borrowed_str(self.parse_str()?)
            }
            _ => Err(Error::InvalidType),
        }
    }

    /// Unsupported. String is not available in no-std.
    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    /// Unsupported
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    /// Unsupported
    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_whitespace().ok_or(Error::EofWhileParsingValue)? {
            b'n' => {
                self.eat_char();
                self.parse_ident(b"ull")?;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek = match self.parse_whitespace() {
            Some(b) => b,
            None => {
                return Err(Error::EofWhileParsingValue);
            }
        };

        match peek {
            b'n' => {
                self.eat_char();
                self.parse_ident(b"ull")?;
                visitor.visit_unit()
            }
            _ => Err(Error::InvalidType),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    /// Unsupported. We can’t parse newtypes because we don’t know the underlying type.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek = self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;

        match peek {
            b'[' => {
                self.eat_char();
                let ret = visitor.visit_seq(SeqAccess::new(self))?;

                self.end_seq()?;

                Ok(ret)
            }
            _ => Err(Error::InvalidType),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let peek = self.parse_whitespace().ok_or(Error::EofWhileParsingValue)?;

        if peek == b'{' {
            self.eat_char();

            let ret = visitor.visit_map(MapAccess::new(self))?;

            self.end_map()?;

            Ok(ret)
        } else {
            Err(Error::InvalidType)
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_whitespace().ok_or(Error::EofWhileParsingValue)? {
            b'"' => visitor.visit_enum(UnitVariantAccess::new(self)),
            b'{' => {
                self.eat_char();
                let value = visitor.visit_enum(VariantAccess::new(self))?;
                match self.parse_whitespace().ok_or(Error::EofWhileParsingValue)? {
                    b'}' => {
                        self.eat_char();
                        Ok(value)
                    }
                    _ => Err(Error::ExpectedSomeValue),
                }
            }
            _ => Err(Error::ExpectedSomeValue),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    /// Used to throw out fields from JSON objects that we don’t want to
    /// keep in our structs.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.parse_whitespace().ok_or(Error::EofWhileParsingValue)? {
            b'"' => self.deserialize_str(visitor),
            b'[' => self.deserialize_seq(visitor),
            b'{' => self.deserialize_struct("ignored", &[], visitor),
            b',' | b'}' | b']' => Err(Error::ExpectedSomeValue),
            // If it’s something else then we chomp until we get to an end delimiter.
            // This does technically allow for illegal JSON since we’re just ignoring
            // characters rather than parsing them.
            _ => loop {
                match self.peek() {
                    // The visitor is expected to be UnknownAny’s visitor, which
                    // implements visit_unit to return its unit Ok result.
                    Some(b',') | Some(b'}') | Some(b']') => break visitor.visit_unit(),
                    Some(_) => self.eat_char(),
                    None => break Err(Error::EofWhileParsingString),
                }
            },
        }
    }
}

impl de::Error for Error {
    #[cfg_attr(not(feature = "custom-error-messages"), allow(unused_variables))]
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        #[cfg(not(feature = "custom-error-messages"))]
        {
            Error::CustomError
        }
        #[cfg(feature = "custom-error-messages")]
        {
            use core::fmt::Write;

            let mut string = heapless::String::new();
            write!(string, "{:.64}", msg).unwrap();
            Error::CustomErrorWithMessage(string)
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::EofWhileParsingList => "EOF while parsing a list.",
                Error::EofWhileParsingObject => "EOF while parsing an object.",
                Error::EofWhileParsingString => "EOF while parsing a string.",
                Error::EofWhileParsingValue => "EOF while parsing a JSON value.",
                Error::ExpectedColon => "Expected this character to be a `':'`.",
                Error::ExpectedListCommaOrEnd => {
                    "Expected this character to be either a `','` or\
                     a \
                     `']'`."
                }
                Error::ExpectedObjectCommaOrEnd => {
                    "Expected this character to be either a `','` \
                     or a \
                     `'}'`."
                }
                Error::ExpectedSomeIdent => {
                    "Expected to parse either a `true`, `false`, or a \
                     `null`."
                }
                Error::ExpectedSomeValue => "Expected this character to start a JSON value.",
                Error::InvalidNumber => "Invalid number.",
                Error::InvalidType => "Invalid type",
                Error::InvalidUnicodeCodePoint => "Invalid unicode code point.",
                Error::KeyMustBeAString => "Object key is not a string.",
                Error::TrailingCharacters => {
                    "JSON has non-whitespace trailing characters after \
                     the \
                     value."
                }
                Error::TrailingComma => "JSON has a comma after the last value in an array or map.",
                Error::CustomError => "JSON does not match deserializer’s expected format.",
                #[cfg(feature = "custom-error-messages")]
                Error::CustomErrorWithMessage(msg) => msg.as_str(),
                _ => "Invalid JSON",
            }
        )
    }
}

/// Deserializes an instance of type `T` from bytes of JSON text
/// Returns the value and the number of bytes consumed in the process
pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<(T, usize)>
where
    T: de::Deserialize<'a>,
{
    let mut de = Deserializer::new(v);
    let value = de::Deserialize::deserialize(&mut de)?;
    let length = de.end()?;

    Ok((value, length))
}

/// Deserializes an instance of type T from a string of JSON text
pub fn from_str<'a, T>(s: &'a str) -> Result<(T, usize)>
where
    T: de::Deserialize<'a>,
{
    from_slice(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use serde_derive::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    enum Type {
        #[serde(rename = "boolean")]
        Boolean,
        #[serde(rename = "number")]
        Number,
        #[serde(rename = "thing")]
        Thing,
    }

    #[test]
    fn array() {
        assert_eq!(crate::from_str::<[i32; 0]>("[]"), Ok(([], 2)));
        assert_eq!(crate::from_str("[0, 1, 2]"), Ok(([0, 1, 2], 9)));

        // errors
        assert!(crate::from_str::<[i32; 2]>("[0, 1,]").is_err());
    }

    #[test]
    fn bool() {
        assert_eq!(crate::from_str("true"), Ok((true, 4)));
        assert_eq!(crate::from_str(" true"), Ok((true, 5)));
        assert_eq!(crate::from_str("true "), Ok((true, 5)));

        assert_eq!(crate::from_str("false"), Ok((false, 5)));
        assert_eq!(crate::from_str(" false"), Ok((false, 6)));
        assert_eq!(crate::from_str("false "), Ok((false, 6)));

        // errors
        assert!(crate::from_str::<bool>("true false").is_err());
        assert!(crate::from_str::<bool>("tru").is_err());
    }

    #[test]
    fn floating_point() {
        assert_eq!(crate::from_str("5.0"), Ok((5.0, 3)));
        assert_eq!(crate::from_str("1"), Ok((1.0, 1)));
        assert_eq!(crate::from_str("1e5"), Ok((1e5, 3)));
        assert!(crate::from_str::<f32>("a").is_err());
        assert!(crate::from_str::<f32>(",").is_err());
    }

    #[test]
    fn integer() {
        assert_eq!(crate::from_str("5"), Ok((5, 1)));
        assert_eq!(crate::from_str("101"), Ok((101, 3)));
        assert!(crate::from_str::<u16>("1e5").is_err());
        assert!(crate::from_str::<u8>("256").is_err());
        assert!(crate::from_str::<f32>(",").is_err());
    }

    #[test]
    fn enum_clike() {
        assert_eq!(crate::from_str(r#" "boolean" "#), Ok((Type::Boolean, 11)));
        assert_eq!(crate::from_str(r#" "number" "#), Ok((Type::Number, 10)));
        assert_eq!(crate::from_str(r#" "thing" "#), Ok((Type::Thing, 9)));
    }

    #[test]
    fn str() {
        assert_eq!(crate::from_str(r#" "hello" "#), Ok(("hello", 9)));
        assert_eq!(crate::from_str(r#" "" "#), Ok(("", 4)));
        assert_eq!(crate::from_str(r#" " " "#), Ok((" ", 5)));
        assert_eq!(crate::from_str(r#" "👏" "#), Ok(("👏", 8)));

        // no unescaping is done (as documented as a known issue in lib.rs)
        assert_eq!(crate::from_str(r#" "hel\tlo" "#), Ok(("hel\\tlo", 11)));
        assert_eq!(crate::from_str(r#" "hello \\" "#), Ok(("hello \\\\", 12)));

        // escaped " in the string content
        assert_eq!(crate::from_str(r#" "foo\"bar" "#), Ok((r#"foo\"bar"#, 12)));
        assert_eq!(
            crate::from_str(r#" "foo\\\"bar" "#),
            Ok((r#"foo\\\"bar"#, 14))
        );
        assert_eq!(
            crate::from_str(r#" "foo\"\"bar" "#),
            Ok((r#"foo\"\"bar"#, 14))
        );
        assert_eq!(crate::from_str(r#" "\"bar" "#), Ok((r#"\"bar"#, 9)));
        assert_eq!(crate::from_str(r#" "foo\"" "#), Ok((r#"foo\""#, 9)));
        assert_eq!(crate::from_str(r#" "\"" "#), Ok((r#"\""#, 6)));

        // non-excaped " preceded by backslashes
        assert_eq!(
            crate::from_str(r#" "foo bar\\" "#),
            Ok((r#"foo bar\\"#, 13))
        );
        assert_eq!(
            crate::from_str(r#" "foo bar\\\\" "#),
            Ok((r#"foo bar\\\\"#, 15))
        );
        assert_eq!(
            crate::from_str(r#" "foo bar\\\\\\" "#),
            Ok((r#"foo bar\\\\\\"#, 17))
        );
        assert_eq!(
            crate::from_str(r#" "foo bar\\\\\\\\" "#),
            Ok((r#"foo bar\\\\\\\\"#, 19))
        );
        assert_eq!(crate::from_str(r#" "\\" "#), Ok((r#"\\"#, 6)));
    }

    #[test]
    fn struct_bool() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Led {
            led: bool,
        }

        assert_eq!(
            crate::from_str(r#"{ "led": true }"#),
            Ok((Led { led: true }, 15))
        );
        assert_eq!(
            crate::from_str(r#"{ "led": false }"#),
            Ok((Led { led: false }, 16))
        );
    }

    #[test]
    fn struct_with_array_field() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Test {
            status: bool,
            point: [u32; 3],
        }

        assert_eq!(
            crate::from_str(r#"{ "status": true, "point": [1,2,3] }"#),
            Ok((Test {
                status: true,
                point: [1_u32, 2, 3]
            }, 36))
        );
    }

    #[test]
    fn struct_with_tuple_field() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Test {
            status: bool,
            point: (u32, u32, u32),
        }

        assert_eq!(
            crate::from_str(r#"{ "status": true, "point": [1,2,3] }"#),
            Ok((Test {
                status: true,
                point: (1_u32, 2, 3)
            }, 36))
        );
    }

    #[test]
    fn struct_i8() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Temperature {
            temperature: i8,
        }

        assert_eq!(
            crate::from_str(r#"{ "temperature": -17 }"#),
            Ok((Temperature { temperature: -17 }, 22))
        );

        assert_eq!(
            crate::from_str(r#"{ "temperature": -0 }"#),
            Ok((Temperature { temperature: -0 }, 21))
        );

        assert_eq!(
            crate::from_str(r#"{ "temperature": 0 }"#),
            Ok((Temperature { temperature: 0 }, 20))
        );

        // out of range
        assert!(crate::from_str::<Temperature>(r#"{ "temperature": 128 }"#).is_err());
        assert!(crate::from_str::<Temperature>(r#"{ "temperature": -129 }"#).is_err());
    }

    #[test]
    fn struct_f32() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Temperature {
            temperature: f32,
        }

        assert_eq!(
            crate::from_str(r#"{ "temperature": -17.2 }"#),
            Ok((Temperature { temperature: -17.2 }, 24))
        );

        assert_eq!(
            crate::from_str(r#"{ "temperature": -0.0 }"#),
            Ok((Temperature { temperature: -0. }, 23))
        );

        assert_eq!(
            crate::from_str(r#"{ "temperature": -2.1e-3 }"#),
            Ok((
                Temperature {
                    temperature: -2.1e-3
                },
                26
            ))
        );

        assert_eq!(
            crate::from_str(r#"{ "temperature": -3 }"#),
            Ok((Temperature { temperature: -3. }, 21))
        );

        use core::f32;

        assert_eq!(
            crate::from_str(r#"{ "temperature": -1e500 }"#),
            Ok((
                Temperature {
                    temperature: f32::NEG_INFINITY
                },
                25
            ))
        );

        assert!(crate::from_str::<Temperature>(r#"{ "temperature": 1e1e1 }"#).is_err());
        assert!(crate::from_str::<Temperature>(r#"{ "temperature": -2-2 }"#).is_err());
        assert!(crate::from_str::<Temperature>(r#"{ "temperature": 1 1 }"#).is_err());
        assert!(crate::from_str::<Temperature>(r#"{ "temperature": 0.0. }"#).is_err());
        assert!(crate::from_str::<Temperature>(r#"{ "temperature": ä }"#).is_err());
    }

    #[test]
    fn struct_option() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Property<'a> {
            #[serde(borrow)]
            description: Option<&'a str>,
        }

        assert_eq!(
            crate::from_str(r#"{ "description": "An ambient temperature sensor" }"#),
            Ok((
                Property {
                    description: Some("An ambient temperature sensor"),
                },
                50
            ))
        );

        assert_eq!(
            crate::from_str(r#"{ "description": null }"#),
            Ok((Property { description: None }, 23))
        );

        assert_eq!(
            crate::from_str(r#"{}"#),
            Ok((Property { description: None }, 2))
        );
    }

    #[test]
    fn struct_u8() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Temperature {
            temperature: u8,
        }

        assert_eq!(
            crate::from_str(r#"{ "temperature": 20 }"#),
            Ok((Temperature { temperature: 20 }, 21))
        );

        assert_eq!(
            crate::from_str(r#"{ "temperature": 0 }"#),
            Ok((Temperature { temperature: 0 }, 20))
        );

        // out of range
        assert!(crate::from_str::<Temperature>(r#"{ "temperature": 256 }"#).is_err());
        assert!(crate::from_str::<Temperature>(r#"{ "temperature": -1 }"#).is_err());
    }

    #[test]
    fn test_unit() {
        assert_eq!(crate::from_str::<()>(r#"null"#), Ok(((), 4)));
    }

    #[test]
    fn newtype_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct A(pub u32);

        assert_eq!(crate::from_str::<A>(r#"54"#), Ok((A(54), 2)));
    }

    #[test]
    fn test_newtype_variant() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum A {
            A(u32),
        }
        let a = A::A(54);
        let x = crate::from_str::<A>(r#"{"A":54}"#);
        assert_eq!(x, Ok((a, 8)));
    }

    #[test]
    fn test_struct_variant() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum A {
            A { x: u32, y: u16 },
        }
        let a = A::A { x: 54, y: 720 };
        let x = crate::from_str::<A>(r#"{"A": {"x":54,"y":720 } }"#);
        assert_eq!(x, Ok((a, 25)));
    }

    #[test]
    #[cfg(not(feature = "custom-error-messages"))]
    fn struct_tuple() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Xy(i8, i8);

        assert_eq!(crate::from_str(r#"[10, 20]"#), Ok((Xy(10, 20), 8)));
        assert_eq!(crate::from_str(r#"[10, -20]"#), Ok((Xy(10, -20), 9)));

        // wrong number of args
        assert_eq!(
            crate::from_str::<Xy>(r#"[10]"#),
            Err(crate::de::Error::CustomError)
        );
        assert_eq!(
            crate::from_str::<Xy>(r#"[10, 20, 30]"#),
            Err(crate::de::Error::TrailingCharacters)
        );
    }

    #[test]
    #[cfg(feature = "custom-error-messages")]
    fn struct_tuple() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Xy(i8, i8);

        assert_eq!(crate::from_str(r#"[10, 20]"#), Ok((Xy(10, 20), 8)));
        assert_eq!(crate::from_str(r#"[10, -20]"#), Ok((Xy(10, -20), 9)));

        // wrong number of args
        assert_eq!(
            crate::from_str::<Xy>(r#"[10]"#),
            Err(crate::de::Error::CustomErrorWithMessage(
                "invalid length 1, expected tuple struct Xy with 2 elements".into()
            ))
        );
        assert_eq!(
            crate::from_str::<Xy>(r#"[10, 20, 30]"#),
            Err(crate::de::Error::TrailingCharacters)
        );
    }

    #[test]
    fn ignoring_extra_fields() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Temperature {
            temperature: u8,
        }

        assert_eq!(
            crate::from_str(r#"{ "temperature": 20, "high": 80, "low": -10, "updated": true }"#),
            Ok((Temperature { temperature: 20 }, 62))
        );

        assert_eq!(
            crate::from_str(
                r#"{ "temperature": 20, "conditions": "windy", "forecast": "cloudy" }"#
            ),
            Ok((Temperature { temperature: 20 }, 66))
        );

        assert_eq!(
            crate::from_str(r#"{ "temperature": 20, "hourly_conditions": ["windy", "rainy"] }"#),
            Ok((Temperature { temperature: 20 }, 62))
        );

        assert_eq!(
            crate::from_str(
                r#"{ "temperature": 20, "source": { "station": "dock", "sensors": ["front", "back"] } }"#
            ),
            Ok((Temperature { temperature: 20 }, 84))
        );

        assert_eq!(
            crate::from_str(r#"{ "temperature": 20, "invalid": this-is-ignored }"#),
            Ok((Temperature { temperature: 20 }, 49))
        );

        assert_eq!(
            crate::from_str::<Temperature>(r#"{ "temperature": 20, "broken": }"#),
            Err(crate::de::Error::ExpectedSomeValue)
        );

        assert_eq!(
            crate::from_str::<Temperature>(r#"{ "temperature": 20, "broken": [ }"#),
            Err(crate::de::Error::ExpectedSomeValue)
        );

        assert_eq!(
            crate::from_str::<Temperature>(r#"{ "temperature": 20, "broken": ] }"#),
            Err(crate::de::Error::ExpectedSomeValue)
        );
    }

    #[test]
    #[cfg(feature = "custom-error-messages")]
    fn preserve_short_error_message() {
        use serde::de::Error;
        assert_eq!(
            crate::de::Error::custom("something bad happened"),
            crate::de::Error::CustomErrorWithMessage("something bad happened".into())
        );
    }

    #[test]
    #[cfg(feature = "custom-error-messages")]
    fn truncate_error_message() {
        use serde::de::Error;
        assert_eq!(
            crate::de::Error::custom("0123456789012345678901234567890123456789012345678901234567890123 <- after here the message should be truncated"),
            crate::de::Error::CustomErrorWithMessage(
                "0123456789012345678901234567890123456789012345678901234567890123".into()
            )
        );
    }

    // See https://iot.mozilla.org/wot/#thing-resource
    #[test]
    fn wot() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Thing<'a> {
            #[serde(borrow)]
            properties: Properties<'a>,
            #[serde(rename = "type")]
            ty: Type,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Properties<'a> {
            #[serde(borrow)]
            temperature: Property<'a>,
            #[serde(borrow)]
            humidity: Property<'a>,
            #[serde(borrow)]
            led: Property<'a>,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Property<'a> {
            #[serde(rename = "type")]
            ty: Type,
            unit: Option<&'a str>,
            #[serde(borrow)]
            description: Option<&'a str>,
            href: &'a str,
        }

        assert_eq!(
            crate::from_str::<Thing<'_>>(
                r#"
                    {
                    "type": "thing",
                    "properties": {
                        "temperature": {
                        "type": "number",
                        "unit": "celsius",
                        "description": "An ambient temperature sensor",
                        "href": "/properties/temperature"
                        },
                        "humidity": {
                        "type": "number",
                        "unit": "percent",
                        "href": "/properties/humidity"
                        },
                        "led": {
                        "type": "boolean",
                        "description": "A red LED",
                        "href": "/properties/led"
                        }
                    }
                    }
                    "#
            ),
            Ok((
                Thing {
                    properties: Properties {
                        temperature: Property {
                            ty: Type::Number,
                            unit: Some("celsius"),
                            description: Some("An ambient temperature sensor"),
                            href: "/properties/temperature",
                        },
                        humidity: Property {
                            ty: Type::Number,
                            unit: Some("percent"),
                            description: None,
                            href: "/properties/humidity",
                        },
                        led: Property {
                            ty: Type::Boolean,
                            unit: None,
                            description: Some("A red LED"),
                            href: "/properties/led",
                        },
                    },
                    ty: Type::Thing,
                },
                852
            ))
        )
    }
}
