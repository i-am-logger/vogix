# Changelog

All notable changes to Vogix will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.3](https://github.com/i-am-logger/vogix/compare/vogix-v0.6.2...vogix-v0.6.3) (2026-03-18)


### Bug Fixes

* deduplicate all flake.lock inputs and fix theme import ([#163](https://github.com/i-am-logger/vogix/issues/163)) ([0ae2f7f](https://github.com/i-am-logger/vogix/commit/0ae2f7f59ad749e6db3bcf9aa68751c2d24b4f1a))

## [0.6.2](https://github.com/i-am-logger/vogix/compare/vogix-v0.6.1...vogix-v0.6.2) (2026-03-17)


### Bug Fixes

* deduplicate nixpkgs in flake.lock via follows ([#161](https://github.com/i-am-logger/vogix/issues/161)) ([24ad2a2](https://github.com/i-am-logger/vogix/commit/24ad2a2e8e33a21887207612bca18404445fd29a))

## [0.6.1](https://github.com/i-am-logger/vogix/compare/vogix-v0.6.0...vogix-v0.6.1) (2026-01-25)


### Bug Fixes

* **nix:** simplify option names and add reusable overlay ([20bffa1](https://github.com/i-am-logger/vogix/commit/20bffa13731e4ef74566265980d1964dc42af627))

## [0.6.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.5.1...vogix-v0.6.0) (2026-01-20)


### Features

* **architecture:** implement template-based theme system ([#140](https://github.com/i-am-logger/vogix/issues/140)) ([9de7493](https://github.com/i-am-logger/vogix/commit/9de749349c39010d4ebd297990ff3ef4c1e32162))

## [0.5.1](https://github.com/i-am-logger/vogix/compare/vogix-v0.5.0...vogix-v0.5.1) (2026-01-19)


### Bug Fixes

* **deps:** update flake inputs ([e13f045](https://github.com/i-am-logger/vogix/commit/e13f045c08271a2790cb6eeeed02466c225609cb))

## [0.5.0](https://github.com/i-am-logger/vogix/compare/vogix-v0.4.0...vogix-v0.5.0) (2026-01-19)


### Features

* add multi-scheme support for base16, base24, ansi16 ([122354b](https://github.com/i-am-logger/vogix/commit/122354bcf38b1a52ff0e335349d0812b12dc58e0))
* **app:** add ripgrep theme support ([#112](https://github.com/i-am-logger/vogix/issues/112)) ([1451178](https://github.com/i-am-logger/vogix/commit/145117824302d89c694424a1dd8072856aa449c7))
* **devenv:** add treefmt-nix for unified formatting ([#126](https://github.com/i-am-logger/vogix/issues/126)) ([574951a](https://github.com/i-am-logger/vogix/commit/574951a0fa9f8ee661629141ac4557c8ba834aaf)), closes [#125](https://github.com/i-am-logger/vogix/issues/125)
* integrate devenv with flake and optimize CI workflow ([#110](https://github.com/i-am-logger/vogix/issues/110)) ([b3480aa](https://github.com/i-am-logger/vogix/commit/b3480aa7dff2f1cad54f8d123498bb3b5d21d491))
* integrate with home-manager settings system ([#115](https://github.com/i-am-logger/vogix/issues/115)) ([45c9c96](https://github.com/i-am-logger/vogix/commit/45c9c96ffbc257a6b02bf1ed03ae9985a5719982)), closes [#114](https://github.com/i-am-logger/vogix/issues/114)
* vogix16 runtime theme management for NixOS ([584c9f3](https://github.com/i-am-logger/vogix/commit/584c9f3ddbb519fd6869e3e4259b7819da2028c1))


### Bug Fixes

* **ci:** upgrade cache-nix-action to v7 and add release-please manifest ([3ee3adf](https://github.com/i-am-logger/vogix/commit/3ee3adf546d65c40b6d1afa67a12b25cb43150a2))
* correct version bump - templates removal is not breaking ([5960fe0](https://github.com/i-am-logger/vogix/commit/5960fe021f46a66b2909570852dad2084ac87b55))
* make VM tests generic and auto-discovering ([#108](https://github.com/i-am-logger/vogix/issues/108)) ([7138374](https://github.com/i-am-logger/vogix/commit/7138374c28a4e52a082e3a8f87cc12ae17447f17))
* **reload:** remove shell command injection in touch reload ([#122](https://github.com/i-am-logger/vogix/issues/122)) ([fd961b7](https://github.com/i-am-logger/vogix/commit/fd961b70b0bb56789092360905a23abc000f367f))
* resolve theme loading issues and improve development workflow ([946e62b](https://github.com/i-am-logger/vogix/commit/946e62b626b9465d128030dd8bbd79436a036021))
* resolve theme loading issues and improve development workflow ([da38cad](https://github.com/i-am-logger/vogix/commit/da38cad5b974da8ebfc84b9cb8583dc6dc4c1652))
* test searched for vogix16 bin rather then vogix ([69c488c](https://github.com/i-am-logger/vogix/commit/69c488c0c570bb778cb0027e8763a283a4d48ecf))

## [0.4.0](https://github.com/i-am-logger/vogix/compare/v0.3.1...v0.4.0) (2026-01-17)


### Features

* **devenv:** add treefmt-nix for unified formatting ([#126](https://github.com/i-am-logger/vogix/issues/126)) ([574951a](https://github.com/i-am-logger/vogix/commit/574951a0fa9f8ee661629141ac4557c8ba834aaf)), closes [#125](https://github.com/i-am-logger/vogix/issues/125)

## [0.3.1](https://github.com/i-am-logger/vogix/compare/v0.3.0...v0.3.1) (2026-01-15)


### Bug Fixes

* **reload:** remove shell command injection in touch reload ([#122](https://github.com/i-am-logger/vogix/issues/122)) ([fd961b7](https://github.com/i-am-logger/vogix/commit/fd961b70b0bb56789092360905a23abc000f367f))

## [0.3.0](https://github.com/i-am-logger/vogix/compare/v0.2.0...v0.3.0) (2025-12-03)


### Features

* **app:** add ripgrep theme support ([#112](https://github.com/i-am-logger/vogix/issues/112)) ([1451178](https://github.com/i-am-logger/vogix/commit/145117824302d89c694424a1dd8072856aa449c7))

## [0.2.0](https://github.com/i-am-logger/vogix/compare/v0.1.3...v0.2.0) (2025-12-03)


### Features

* integrate devenv with flake and optimize CI workflow ([#110](https://github.com/i-am-logger/vogix/issues/110)) ([b3480aa](https://github.com/i-am-logger/vogix/commit/b3480aa7dff2f1cad54f8d123498bb3b5d21d491))

## [0.1.3](https://github.com/i-am-logger/vogix/compare/v0.1.2...v0.1.3) (2025-12-03)


### Bug Fixes

* make VM tests generic and auto-discovering ([#108](https://github.com/i-am-logger/vogix/issues/108)) ([7138374](https://github.com/i-am-logger/vogix/commit/7138374c28a4e52a082e3a8f87cc12ae17447f17))

## [0.1.2](https://github.com/i-am-logger/vogix/compare/v0.1.1...v0.1.2) (2025-12-03)


### Bug Fixes

* correct version bump - templates removal is not breaking ([5960fe0](https://github.com/i-am-logger/vogix/commit/5960fe021f46a66b2909570852dad2084ac87b55))
* resolve theme loading issues and improve development workflow ([946e62b](https://github.com/i-am-logger/vogix/commit/946e62b626b9465d128030dd8bbd79436a036021))
* resolve theme loading issues and improve development workflow ([da38cad](https://github.com/i-am-logger/vogix/commit/da38cad5b974da8ebfc84b9cb8583dc6dc4c1652))

## [0.1.1](https://github.com/i-am-logger/vogix/compare/v0.1.0...v0.1.1) (2025-12-02)


### Bug Fixes

* test searched for vogix16 bin rather then vogix ([69c488c](https://github.com/i-am-logger/vogix/commit/69c488c0c570bb778cb0027e8763a283a4d48ecf))

## 0.1.0 (2025-12-02)


### Features

* vogix16 runtime theme management for NixOS ([584c9f3](https://github.com/i-am-logger/vogix/commit/584c9f3ddbb519fd6869e3e4259b7819da2028c1))

## [Unreleased]

### Added
- GitHub Actions CI/CD workflows
- Comprehensive CONTRIBUTING.md with development guidelines
- Automated release process with release-please

### Changed
- Updated all documentation to reflect actual Nix-based architecture
- Corrected README examples to show proper home-manager integration
- Binary name consistently referred to as `vogix` throughout docs

### Removed
- Obsolete ARCHITECTURE-REDESIGN.md documentation

## [0.5.0] - 2024-XX-XX

### Added
- Renamed binary from `vogix16` to `vogix` for consistency
- Auto-toggle `switch` command (no arguments needed)
- Per-app theme and variant override support
- Comprehensive theme library with 19 themes
- Console (TTY) theme support with setvtrgb integration

### Changed
- Refactored architecture: Nix generates all theme configs at build time
- CLI now only manages symlinks, not config generation
- Improved symlink architecture for instant theme switching
- Enhanced state persistence

## [0.4.0] - 2024-XX-XX

### Added
- Comprehensive automated testing suite (16 test scenarios)
- VM-based integration testing
- Theme validation at build time
- Semantic color API for application modules

### Changed
- Improved error handling across all components
- Enhanced reload mechanisms with multiple methods
- Better symlink management

## [0.3.0] - 2024-XX-XX

### Added
- Auto-discovery of themes from `themes/` directory
- Auto-discovery of application generators
- Systemd service for runtime directory setup
- Support for per-app configuration overrides

### Changed
- Migrated from runtime template processing to build-time generation
- Simplified CLI to focus on symlink management

## [0.2.0] - 2024-XX-XX

### Added
- Multiple application reload methods (touch, signal, command)
- Shell completions for all major shells
- State persistence for current theme/variant

### Changed
- Improved NixOS integration
- Enhanced home-manager module

## [0.1.0] - 2024-XX-XX

### Added
- Initial release
- vogix16 design system with 16-color palette
- Basic theme switching functionality
- Dark and light variant support
- NixOS and home-manager integration
- Example themes (aikido, forest, matrix)
- CLI tool for runtime theme management
- Application reload mechanism
- Documentation for design system, architecture, and usage

[Unreleased]: https://github.com/i-am-logger/vogix/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.5.0
[0.4.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.4.0
[0.3.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.3.0
[0.2.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.2.0
[0.1.0]: https://github.com/i-am-logger/vogix/releases/tag/v0.1.0
