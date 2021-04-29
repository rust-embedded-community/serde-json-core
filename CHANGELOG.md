# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
### Added
- `heapless` is now publicly exported

### Changed
- Floating point numbers terminated by EOF may now be deserialized
- [ryu](https://github.com/dtolnay/ryu) is used to serialize `f32` and `f64`
- Added missing implementation for `serialize_bytes` method
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

[Unreleased]: https://github.com/rust-embedded-community/serde-json-core/compare/v0.2.0...HEAD
[v0.2.0]: https://github.com/rust-embedded-community/serde-json-core/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/rust-embedded-community/serde-json-core/releases/tag/v0.1.0
