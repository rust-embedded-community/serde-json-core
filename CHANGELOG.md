# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Added
- Support for serialization into slices
- Support for serializing and deserializing unit types, newtypes and variants

### Changed
- Changed deserializations to return the number of bytes used
- Changed deserializer and serializer to handle escaped strings
- Changed deserializer to handle whitespaces before sequence

## [v0.1.0] - 2019-11-17

### Added
- support for floats and tuple structs

### Changed
- [breaking-change] The `heapless` dependency has been bumped to v0.5.0
- This crate now compiles on stable (MSRV = 1.31)

## v0.0.1

Initial release

[Unreleased]: https://github.com/rust-embedded/cortex-m/compare/v0.5.8...HEAD
