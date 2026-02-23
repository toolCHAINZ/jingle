# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.7](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.4.6...jingle_sleigh-v0.4.7) - 2026-02-22

### Other

- has_address method to ImageSections ([#192](https://github.com/toolCHAINZ/jingle/pull/192))

## [0.4.6](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.4.5...jingle_sleigh-v0.4.6) - 2026-02-19

### Added

- add readability, writability, and executability check helpers ([#188](https://github.com/toolCHAINZ/jingle/pull/188))
- trait refactor ([#187](https://github.com/toolCHAINZ/jingle/pull/187))

### Fixed

- Various bug fixes and features ([#185](https://github.com/toolCHAINZ/jingle/pull/185))

## [0.4.5](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.4.4...jingle_sleigh-v0.4.5) - 2026-02-13

### Other

- handle extrapop adjustment for Call operations in valuation state ([#177](https://github.com/toolCHAINZ/jingle/pull/177))
- stack pointer and program counter support to Sleigh context ([#176](https://github.com/toolCHAINZ/jingle/pull/176))
- Implement JingleDisplay for SingleValuation types ([#171](https://github.com/toolCHAINZ/jingle/pull/171))

## [0.4.4](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.4.3...jingle_sleigh-v0.4.4) - 2026-02-02

### Other

- generalize display impls, rename trait, move it to jingle_sleigh ([#151](https://github.com/toolCHAINZ/jingle/pull/151))

## [0.4.3](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.4.2...jingle_sleigh-v0.4.3) - 2026-02-01

### Added

- add an api to get a sub-graph ([#141](https://github.com/toolCHAINZ/jingle/pull/141))

## [0.4.2](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.4.1...jingle_sleigh-v0.4.2) - 2026-01-28

### Added

- specialized varnode map data structure, analysis refactor ([#134](https://github.com/toolCHAINZ/jingle/pull/134))
- Allow Location Lattice Elements in CFGs ([#132](https://github.com/toolCHAINZ/jingle/pull/132))

### Other

- Trait Cleanup and PcodeOpRef ([#133](https://github.com/toolCHAINZ/jingle/pull/133))
- Analysis Refactoring ([#129](https://github.com/toolCHAINZ/jingle/pull/129))

## [0.4.1](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.4.0...jingle_sleigh-v0.4.1) - 2025-12-21

### Fixed

- allow blank lines and extra whitespace in pcode parsing ([#125](https://github.com/toolCHAINZ/jingle/pull/125))

## [0.4.0](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.3.3...jingle_sleigh-v0.4.0) - 2025-12-19

### Added

- [**breaking**] allow parsing pcode from a string ([#123](https://github.com/toolCHAINZ/jingle/pull/123))

## [0.3.3](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.3.2...jingle_sleigh-v0.3.3) - 2025-12-09

### Added

- allow annotating CALL and CALLOTHER with metadata ([#118](https://github.com/toolCHAINZ/jingle/pull/118))

### Other

- bump to ghidra 12 ([#122](https://github.com/toolCHAINZ/jingle/pull/122))

## [0.3.2](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.3.1...jingle_sleigh-v0.3.2) - 2025-12-03

### Other

- update pyo3 dependency to version 0.27.2 ([#117](https://github.com/toolCHAINZ/jingle/pull/117))
- Disable default features and enable macros for pyo3 dependency ([#115](https://github.com/toolCHAINZ/jingle/pull/115))

## [0.3.1](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.3.0...jingle_sleigh-v0.3.1) - 2025-12-02

### Added

- CTL Model Checking ([#114](https://github.com/toolCHAINZ/jingle/pull/114))

## [0.3.0](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.2.4...jingle_sleigh-v0.3.0) - 2025-09-19

### Changed

- [**breaking**] Remove unneeded traits and types ([#93](https://github.com/toolCHAINZ/jingle/pull/93))

### Other

- add varnode fn ([#96](https://github.com/toolCHAINZ/jingle/pull/96))

## [0.2.4](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.2.3...jingle_sleigh-v0.2.4) - 2025-09-17

### Added

- add bounded visitor analysis ([#77](https://github.com/toolCHAINZ/jingle/pull/77))

### Other

- bump pyo3 ([#92](https://github.com/toolCHAINZ/jingle/pull/92))

## [0.2.3](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.2.2...jingle_sleigh-v0.2.3) - 2025-08-21

### Other

- update z3 ([#84](https://github.com/toolCHAINZ/jingle/pull/84))

## [0.2.1](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.2.0...jingle_sleigh-v0.2.1) - 2025-08-12

### Fixed

- consolidate one-off display types ([#81](https://github.com/toolCHAINZ/jingle/pull/81))

## [0.2.0](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.1.4...jingle_sleigh-v0.2.0) - 2025-08-06

### Added

- add basic analysis ([#74](https://github.com/toolCHAINZ/jingle/pull/74))

### Other

- [**breaking**] bump z3 ([#78](https://github.com/toolCHAINZ/jingle/pull/78))

## [0.1.2](https://github.com/toolCHAINZ/jingle/compare/jingle_sleigh-v0.1.1...jingle_sleigh-v0.1.2) - 2025-07-10

### Other

- add release-plz ([#61](https://github.com/toolCHAINZ/jingle/pull/61))
- Bump ghidra to 11.4 ([#57](https://github.com/toolCHAINZ/jingle/pull/57))
- Concretization tweaks ([#56](https://github.com/toolCHAINZ/jingle/pull/56))
- Add Python Type Annotations ([#53](https://github.com/toolCHAINZ/jingle/pull/53))
- Rust Edition 2024 ([#47](https://github.com/toolCHAINZ/jingle/pull/47))
- Expose more structs ([#49](https://github.com/toolCHAINZ/jingle/pull/49))
- Fill out more python APIs ([#43](https://github.com/toolCHAINZ/jingle/pull/43))
- Bump deps ([#42](https://github.com/toolCHAINZ/jingle/pull/42))
- Add missing cfg guard
- Python Bindings ([#37](https://github.com/toolCHAINZ/jingle/pull/37))
- API Cleanup ([#35](https://github.com/toolCHAINZ/jingle/pull/35))
- Target ghidra 11.3 ([#32](https://github.com/toolCHAINZ/jingle/pull/32))
- Merge dev changes ([#31](https://github.com/toolCHAINZ/jingle/pull/31))
