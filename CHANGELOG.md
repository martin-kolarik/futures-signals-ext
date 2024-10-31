# Changelog

All notable changes to this project will be documented in this file. The
format is based on [Keep a Changelog], and this project aims to follow
[Semantic Versioning].

## [0.4.0] - 2024-05-07

### Changed

- `return_lease` renamed to `commit`

## [0.2.0] - 2024-05-07

### Added

- `MutableVecEntry` trait added

### Changed

- binary searching methods removed

## [0.1.9] - 2024-04-13

### Added

- `MutableVec::replace` added

### Changed

- `MutableVec::replace_or_extend` renamed to `MutableVec::replace_or_extend_keyed`

## [0.1.8] - 2024-02-29

### Changed

- async code moved to artwrap crate

## [0.1.7] - 2024-02-27

### Changed

- bump up dependencies versions

## [0.1.6] - 2024-02-14

### Added

- `MutableVecExt::enumerate_map` added

- `MutableVecExt::find_*` and `filter_*` added

- `SignalExtMapOption::unwrap_or_default` added

- `SignalSpawn::spawn_fut` and `SignalVecSpawn::spawn_fut` added

## [0.1.5] - 2023-10-23

### Added

- `MutableVecExt::find_set_*` and `find_remove` added

- `MutableVecExt::extend_*` and `replace_*` added

- `SignalVecFinalizerExt::is_empty`, `len`, `all` and `any` added

### Changed

- `SignalVecItemExt` renamed to `SignalVecFirstExt`

## [0.1.4] - 2023-08-20

### Added

- Introduce wasm environment, `wasm-bindgen-futures` used for spawning tasks.

- `MutableOption::empty_if_contains` changed to `MutableOption::take_if_value`

## [0.1.3] - 2023-08-20

### Fixed

- `cargo.toml` features cleanup and fixes

### Changed

- `MutableOption::empty_if_contains` changed to `MutableOption::take_if_value`
