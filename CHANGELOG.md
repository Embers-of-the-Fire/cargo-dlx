# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1](https://github.com/Embers-of-the-Fire/cargo-dlx/compare/v0.4.0...v0.4.1) - 2026-03-13

### Added

- Add --bin, --example ([#19](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/19))

### Other

- *(design)* Call out the cache is mostly a two-way door ([#23](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/23))
- Remove --no-package-cache ([#22](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/22))
- *(design)* Integrate prior art into alternatives ([#21](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/21))
- *(design)* Explore prior art ([#20](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/20))
- Leverage snapshot testing ([#18](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/18))
- Organize bin, lib, and tests more like Cargo ([#16](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/16))

## [0.4.0](https://github.com/Embers-of-the-Fire/cargo-dlx/compare/v0.3.0...v0.4.0) - 2026-03-11

### Added

- *(cli)* Add `--clear` option for garbage collection ([#9](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/9))
- Add --profile support ([#13](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/13))

### Fixed

- *(help)* Be consistent with Cargo's headings ([#10](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/10))

### Other

- *(design)* Clarify and expan design document ([#14](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/14))
- *(cache)* [**breaking**] refactor cache storage directory ([#12](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/12))

## [0.3.0](https://github.com/Embers-of-the-Fire/cargo-dlx/compare/v0.2.0...v0.3.0) - 2026-03-09

### Added

- *(cli)* [**breaking**] remove -c shell execution flag
- add rich package reference syntax

### Other

- *(cli)* remove relative file url support
- *(lint)* fix clippy lint
- add design docs
- *(cli)* Reuse clap's help ([#5](https://github.com/Embers-of-the-Fire/cargo-dlx/pull/5))
- *(ci)* add release-plz ci
