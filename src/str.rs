#[derive(Debug)]
pub enum EscapedStringFragment<'a> {
    NotEscaped(&'a str),
    Escaped(char),
}

#[derive(Debug)]
pub enum StringUnescapeError {
    InvalidEscapeSequence,
}

fn unescape_next_fragment(
    escaped_string: &str,
) -> Result<(EscapedStringFragment<'_>, &str), StringUnescapeError> {
    Ok(if let Some(rest) = escaped_string.strip_prefix('\\') {
        let mut escaped_string_chars = rest.chars();

        let unescaped_char = match escaped_string_chars.next() {
            Some('"') => '"',
            Some('\\') => '\\',
            Some('/') => '/',
            Some('b') => '\x08',
            Some('f') => '\x0C',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('u') => {
                fn split_first_slice(s: &str, len: usize) -> Option<(&str, &str)> {
                    Some((s.get(..len)?, s.get(len..)?))
                }

                let (escape_sequence, remaining_escaped_string_chars) =
                    split_first_slice(escaped_string_chars.as_str(), 4)
                        .ok_or(StringUnescapeError::InvalidEscapeSequence)?;

                escaped_string_chars = remaining_escaped_string_chars.chars();

                u32::from_str_radix(escape_sequence, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or(StringUnescapeError::InvalidEscapeSequence)?
            }
            _ => return Err(StringUnescapeError::InvalidEscapeSequence),
        };

        (
            EscapedStringFragment::Escaped(unescaped_char),
            escaped_string_chars.as_str(),
        )
    } else {
        let (fragment, rest) =
            escaped_string.split_at(escaped_string.find('\\').unwrap_or(escaped_string.len()));

        (EscapedStringFragment::NotEscaped(fragment), rest)
    })
}

/// A borrowed escaped string
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename = "__serde_json_core_escaped_string__")]
pub struct EscapedStr<'a>(pub &'a str);

impl<'a> EscapedStr<'a> {
    pub(crate) const NAME: &'static str = "__serde_json_core_escaped_string__";

    pub fn fragments(&self) -> EscapedStringFragmentIter<'a> {
        EscapedStringFragmentIter(self.0)
    }
}

pub struct EscapedStringFragmentIter<'a>(&'a str);

impl<'a> EscapedStringFragmentIter<'a> {
    pub fn as_str(&self) -> EscapedStr<'a> {
        EscapedStr(self.0)
    }
}

impl<'a> Iterator for EscapedStringFragmentIter<'a> {
    type Item = Result<EscapedStringFragment<'a>, StringUnescapeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        Some(unescape_next_fragment(self.0).map(|(fragment, rest)| {
            self.0 = rest;

            fragment
        }))
    }
}
