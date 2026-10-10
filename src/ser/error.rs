//! Error type for handling both IO errors and string-formatting errors ([`core::format_args!`])
//!
//! The [`Error`] type basically recreates the [`embedded_io::WriteFmtError`] error and implements
//! on it additional traits required by the various `serde` methods.

use core::fmt;

use embedded_io::WriteFmtError;

/// This type represents all possible errors that can occur when serializing JSON data
#[derive(Debug)]
pub enum Error<E> {
    /// An I/O error occurred while writing JSON data
    Io(E),
    /// A formatting error occurred while writing JSON data
    ///
    /// Can only occur when calling [`crate::Serializer::collect_str()`]
    Fmt,
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::Io(e)
    }
}

impl<E> From<WriteFmtError<E>> for Error<E> {
    fn from(e: WriteFmtError<E>) -> Self {
        match e {
            WriteFmtError::Other(e) => Error::Io(e),
            WriteFmtError::FmtError => Error::Fmt,
        }
    }
}

impl<E: fmt::Display> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Fmt => f.write_str("fmt error"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> serde::ser::Error for Error<E> {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        unreachable!()
    }
}

#[cfg(feature = "std")]
impl<E: fmt::Debug + fmt::Display> std::error::Error for Error<E> {}
