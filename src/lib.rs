//! [`serde-json`] for `no_std` programs
//!
//! [`serde-json`]: https://crates.io/crates/serde_json
//!
//! This version of [`serde-json`] is aimed at applications that run on resource constrained
//! devices.
//!
//! # Current features
//!
//! - The error type is a simple C like enum (less overhead, smaller memory footprint)
//! - (De)serialization doesn't require memory allocations
//! - Deserialization of integers doesn't go through `u64`; instead the string is directly parsed
//!   into the requested integer type. This avoids pulling in KBs of compiler intrinsics when
//!   targeting a non 64-bit architecture.
//! - Supports deserialization of:
//!   - `bool`
//!   - Integers
//!   - `str` (This is a zero copy operation.) (\*)
//!   - `Option`
//!   - Arrays
//!   - Tuples
//!   - Structs
//!   - C like enums
//! - Supports serialization (compact format only) of:
//!   - `bool`
//!   - Integers
//!   - `str` (\*\*)
//!   - `Option`
//!   - Arrays
//!   - Tuples
//!   - Structs
//!   - C like enums
//!
//! (\*) Deserialization of strings ignores escaped sequences. Escaped sequences might be supported
//! in the future using a different Serializer as this operation is not zero copy.
//!
//! (\*\*) Serialization of strings doesn't escape stuff. This simply has not been implemented yet.
//!
//! # Planned features
//!
//! - (De)serialization from / into IO objects once `core::io::{Read,Write}` becomes a thing.
//!
//! # Non-features
//!
//! This is explicitly out of scope
//!
//! - Anything that involves dynamic memory allocation
//!   - Like the dynamic [`Value`](https://docs.rs/serde_json/1.0.11/serde_json/enum.Value.html)
//!     type
//!
//! # MSRV
//!
//! This crate is guaranteed to compile on stable Rust 1.31.0 and up. It *might* compile with older
//! versions but that may change in any new patch release.

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod de;
pub mod ser;

#[doc(inline)]
pub use self::de::{from_slice, from_str};
#[doc(inline)]
pub use self::ser::{to_string, to_vec};
