# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Added

- Support for optional package `defmt` which allows for easy conversion for
error types when using tools like `probe-rs` for logging over debuggers.
- Implement `Serializer::collect_str`

### Changed

- `heapless` bumped to v0.8.

## [v0.5.1] - 2023-07-26

### Added

- Support for serializing tuple structs. These are serialized as JSON arrays,
  which matches `serde_json` behaviour.
- `Serializer` and `Deserializer` are now `pub`.
- Added `pub` `Serializer::end()` and `Deserializer::end()`.

### Changed

- Increase MSRV to 1.56.0 to work around dependency-MSRV issues (see #72)

## [v0.5.0] - 2022-11-04

### Changed

- Changed serialization of `f32`/`f64` that are `!is_finite()` (i.e. `NAN`, `INFINITY`,
  `NEG_INFINITY`) to result in JSON `null`. This matches `serde_json` behavior.
- Changed deserialization of JSON `null` where `f32`/`f64` is expected to result in
  the respective `NAN`.
- [breaking-change] increase MSRV to Rust `1.55.0` due to `maybe_uninit_extra`.

## [v0.4.0] - 2021-05-08

### Added

- Support for opting out of heapless integration

### Changed

- [breaking-change] use `const_generics` in `to_string()` and `to_vec()` functions.
- [breaking-change] update to `heapless` `0.7`.
- [breaking-change] increase MSRV to Rust `1.51.0` due to `const_generics`.

## [v0.3.0] - 2021-04-29
### Added
- `heapless` is now publicly exported
- Added new `serialize_bytes` method

### Changed
- Floating point numbers terminated by EOF may now be deserialized
- [ryu](https://github.com/dtolnay/ryu) is used to serialize `f32` and `f64`
- [breaking-change] Heapless dependency updated to 0.6.1

## [v0.2.0] - 2020-12-11
### Added
- Support for serialization into slices
- Support for serializing and deserializing unit types, newtypes and variants

### Changed
- Changed deserializations to return the number of bytes used
- Changed deserializer and serializer to handle escaped strings
- Changed deserializer to handle whitespaces before sequence
- Raised MSRV to 1.40.0 in order to use `non_exhaustive`

## [v0.1.0] - 2019-11-17

### Added
- support for floats and tuple structs

### Changed
- [breaking-change] The `heapless` dependency has been bumped to v0.5.0
- This crate now compiles on stable (MSRV = 1.31)

## v0.0.1

Initial release

[Unreleased]: https://github.com/rust-embedded-community/serde-json-core/compare/v0.5.1...HEAD
[v0.5.1]: https://github.com/rust-embedded-community/serde-json-core/compare/v0.5.0...v0.5.1
[v0.5.0]: https://github.com/rust-embedded-community/serde-json-core/compare/v0.4.0...v0.5.0
[v0.4.0]: https://github.com/rust-embedded-community/serde-json-core/compare/v0.3.0...v0.4.0
[v0.3.0]: https://github.com/rust-embedded-community/serde-json-core/compare/v0.2.0...v0.3.0
[v0.2.0]: https://github.com/rust-embedded-community/serde-json-core/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/rust-embedded-community/serde-json-core/releases/tag/v0.1.0
