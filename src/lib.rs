//! [`serde-json`] for `no_std` programs
//!
//! [`serde-json`]: https://crates.io/crates/serde_json
//!
//! This version of [`serde-json`] is aimed at applications that run on resource constrained
//! devices.
//!
//! ## Example
//! ```
//! # use serde::{Serialize, Deserialize};
//! #[derive(Serialize, Deserialize)]
//! struct Data<'a> {
//!     value: u32,
//!     message: &'a str,
//! }
//!
//! // Serialized JSON data can be easily deserialized into Rust types.
//! let message = b"{\"value\":10,\"message\":\"Hello, World!\"}";
//! let (data, _consumed) = serde_json_core::from_slice::<Data<'_>>(message).unwrap();
//! assert_eq!(data.value, 10);
//! assert_eq!(data.message, "Hello, World!");
//!
//! // Structures can also be serialized into slices or strings.
//! let mut deserialized = [0u8; 256];
//! let len = serde_json_core::to_slice(&data, &mut deserialized[..]).unwrap();
//! assert_eq!(&deserialized[..len], message);
//! ```
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
//!   - Floats
//!   - `str` (This is a zero copy operation when deserializing without de-escaping strings.)
//!   - `Option`
//!   - Arrays
//!   - Tuples
//!   - Structs
//!   - C like enums
//! - Supports serialization (compact format only) of:
//!   - `bool`
//!   - Integers
//!   - Floats
//!   - `str`
//!   - `Option`
//!   - Arrays
//!   - Tuples
//!   - Structs
//!   - C like enums
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
//! # Minimum Supported Rust Version (MSRV)
//!
//! This crate is guaranteed to compile on stable Rust 1.70.0 and up. It *might* compile with older
//! versions but that may change in any new patch release.

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod de;
pub mod ser;
pub mod str;

#[doc(inline)]
pub use self::de::{from_slice, from_slice_escaped, from_str, from_str_escaped};
#[doc(inline)]
pub use self::ser::to_slice;
#[cfg(feature = "heapless")]
pub use self::ser::{to_string, to_vec};

#[cfg(feature = "heapless")]
pub use heapless;
