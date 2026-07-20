use core::fmt;

use serde::ser;

use crate::ser::Error;

/// An unreachable error type
///
/// This type cannot be constructed since it contains [`core::convert::Infallible`]
/// and whenever used, it will always panic with [`core::unreachable!`].
pub struct Unreachable<E> {
    _infallible: core::convert::Infallible,
    _e: core::marker::PhantomData<E>,
}

impl<E> fmt::Debug for Unreachable<E> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!()
    }
}

impl<E> fmt::Display for Unreachable<E> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!()
    }
}

#[cfg(feature = "std")]
impl<E> std::error::Error for Unreachable<E> {}

impl<E> ser::Error for Unreachable<E> {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        unreachable!()
    }
}

impl<E: fmt::Debug + fmt::Display> ser::SerializeTupleVariant for Unreachable<E> {
    type Ok = ();
    type Error = Error<E>;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unreachable!()
    }
}
