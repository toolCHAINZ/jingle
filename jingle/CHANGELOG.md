# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.2](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.4.1...jingle-v0.4.2) - 2025-12-21

### Fixed

- update error message ([#127](https://github.com/toolCHAINZ/jingle/pull/127))

## [0.4.1](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.4.0...jingle-v0.4.1) - 2025-12-21

### Other

- updated the following local packages: jingle_sleigh

## [0.4.0](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.3.7...jingle-v0.4.0) - 2025-12-19

### Added

- [**breaking**] allow parsing pcode from a string ([#123](https://github.com/toolCHAINZ/jingle/pull/123))

## [0.3.7](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.3.6...jingle-v0.3.7) - 2025-12-09

### Added

- allow annotating CALL and CALLOTHER with metadata ([#118](https://github.com/toolCHAINZ/jingle/pull/118))

### Fixed

- avoid revisiting locations in CFG traversal ([#121](https://github.com/toolCHAINZ/jingle/pull/121))

## [0.3.6](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.3.5...jingle-v0.3.6) - 2025-12-03

### Other

- update pyo3 dependency to version 0.27.2 ([#117](https://github.com/toolCHAINZ/jingle/pull/117))
- Disable default features and enable macros for pyo3 dependency ([#115](https://github.com/toolCHAINZ/jingle/pull/115))

## [0.3.5](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.3.4...jingle-v0.3.5) - 2025-12-02

### Added

- CTL Model Checking ([#114](https://github.com/toolCHAINZ/jingle/pull/114))
- use default parameterization for pcodecfg ([#111](https://github.com/toolCHAINZ/jingle/pull/111))

### Other

- exclude examples from crates.io ([#113](https://github.com/toolCHAINZ/jingle/pull/113))

## [0.3.4](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.3.3...jingle-v0.3.4) - 2025-10-23

### Added

- add cfg unwinding analysis and initial SMT modeling ([#107](https://github.com/toolCHAINZ/jingle/pull/107))

### Changed

- reduce location repetition in CPA ([#109](https://github.com/toolCHAINZ/jingle/pull/109))

### Fixed

- remove some unnecessary terms from models ([#110](https://github.com/toolCHAINZ/jingle/pull/110))
- unwinding error fix

## [0.3.3](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.3.2...jingle-v0.3.3) - 2025-09-26

### Other

- bump z3 ([#103](https://github.com/toolCHAINZ/jingle/pull/103))

## [0.3.2](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.3.1...jingle-v0.3.2) - 2025-09-19

### Changed

- rename PythonResolvedVarNode ([#101](https://github.com/toolCHAINZ/jingle/pull/101))

## [0.3.1](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.3.0...jingle-v0.3.1) - 2025-09-19

### Other

- Add some APIs ([#99](https://github.com/toolCHAINZ/jingle/pull/99))

## [0.3.0](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.2.7...jingle-v0.3.0) - 2025-09-19

### Changed

- [**breaking**] Remove unneeded traits and types ([#93](https://github.com/toolCHAINZ/jingle/pull/93))

## [0.2.7](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.2.6...jingle-v0.2.7) - 2025-09-17

### Added

- add bounded visitor analysis ([#77](https://github.com/toolCHAINZ/jingle/pull/77))

### Other

- bump pyo3 ([#92](https://github.com/toolCHAINZ/jingle/pull/92))

## [0.2.6](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.2.5...jingle-v0.2.6) - 2025-09-14

### Other

- bump z3

## [0.2.5](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.2.4...jingle-v0.2.5) - 2025-09-03

### Other

- bump z3 ([#88](https://github.com/toolCHAINZ/jingle/pull/88))

## [0.2.4](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.2.3...jingle-v0.2.4) - 2025-08-22

### Fixed

- python z3 lock tweaks ([#86](https://github.com/toolCHAINZ/jingle/pull/86))

## [0.2.3](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.2.2...jingle-v0.2.3) - 2025-08-21

### Other

- update z3 ([#84](https://github.com/toolCHAINZ/jingle/pull/84))

## [0.2.2](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.2.1...jingle-v0.2.2) - 2025-08-12

### Fixed

- consolidate one-off display types ([#81](https://github.com/toolCHAINZ/jingle/pull/81))

## [0.2.1](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.2.0...jingle-v0.2.1) - 2025-08-11

### Other

- update z3 ([#79](https://github.com/toolCHAINZ/jingle/pull/79))

## [0.2.0](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.1.4...jingle-v0.2.0) - 2025-08-06

### Added

- back edge analysis ([#76](https://github.com/toolCHAINZ/jingle/pull/76))
- add basic analysis ([#74](https://github.com/toolCHAINZ/jingle/pull/74))

### Other

- [**breaking**] bump z3 ([#78](https://github.com/toolCHAINZ/jingle/pull/78))

## [0.1.4](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.1.3...jingle-v0.1.4) - 2025-07-16

### Fixed

- remove faulty logic and deprecate old interfaces ([#72](https://github.com/toolCHAINZ/jingle/pull/72))

## [0.1.3](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.1.2...jingle-v0.1.3) - 2025-07-16

### Fixed

- conditional branch metadata bv size ([#70](https://github.com/toolCHAINZ/jingle/pull/70))

## [0.1.2](https://github.com/toolCHAINZ/jingle/compare/jingle-v0.1.1...jingle-v0.1.2) - 2025-07-10

### Other

- add release-plz ([#61](https://github.com/toolCHAINZ/jingle/pull/61))
- Use latest z3 ([#65](https://github.com/toolCHAINZ/jingle/pull/65))
- Add varnode covering logic ([#59](https://github.com/toolCHAINZ/jingle/pull/59))
- Concretization tweaks ([#56](https://github.com/toolCHAINZ/jingle/pull/56))
- Add method to get instructions from a block in python ([#55](https://github.com/toolCHAINZ/jingle/pull/55))
- Add Python Type Annotations ([#53](https://github.com/toolCHAINZ/jingle/pull/53))
- pub ctx
- Add some [Partial]Eq derives
- fmt
- Improved location concretization
- Extend Python Z3 Interop ([#51](https://github.com/toolCHAINZ/jingle/pull/51))
- Rust Edition 2024 ([#47](https://github.com/toolCHAINZ/jingle/pull/47))
- pub
- Add some methods
- fmt + clippy
- Add another display
- Add display => str
- More python varnode representation tweaks
- fmt + clippy
- Add iter impl
- Fix names
- Change iterator to not work specifically on BVs ([#50](https://github.com/toolCHAINZ/jingle/pull/50))
- Expose more structs ([#49](https://github.com/toolCHAINZ/jingle/pull/49))
- Python/Rust Conversion Trait ([#48](https://github.com/toolCHAINZ/jingle/pull/48))
- Change wrapping of pythong z3
- Pub instr
- Python refactor ([#46](https://github.com/toolCHAINZ/jingle/pull/46))
- Fill out more python APIs ([#43](https://github.com/toolCHAINZ/jingle/pull/43))
- Bump deps ([#42](https://github.com/toolCHAINZ/jingle/pull/42))
- Python Bindings ([#37](https://github.com/toolCHAINZ/jingle/pull/37))
- API Cleanup ([#35](https://github.com/toolCHAINZ/jingle/pull/35))
- Merge dev changes ([#31](https://github.com/toolCHAINZ/jingle/pull/31))
