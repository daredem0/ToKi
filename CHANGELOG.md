# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Added workspace release flow targets for `cargo-release` (`release-dry-run`, `release-execute`) and related installer targets in `Justfile`.
- Added dependency-license hygiene tooling with `cargo-deny`/`cargo-about` configs (`deny.toml`, `about.toml`, `about.hbs`) and helper scripts.
- Added generated third-party license inventory output (`THIRD_PARTY_LICENSES.md`).
- Added CI helper scripts for release checks and artifact metadata (`scripts/verify-tag-version.sh`, `scripts/detect-libc-suffix.sh`).
- Added a `package-crate` CI job that packages all workspace crates and uploads `.crate` artifacts.
- Added a dedicated `toki-schemas` workspace crate that owns canonical JSON schema payloads.

### Changed
- Added shared workspace package metadata for versioning and repository fields to better support multi-crate releases.
- Updated crate package metadata and SPDX identifiers for release/tooling compatibility.
- Updated README workflow docs for release and dependency-license checks.
- Replaced the CI workflow with a multi-job pipeline (`build-debug`, `build-release`, `test`, `clippy`, `fmt`, `coverage`, `docs`, `release`, `deploy-pages`) modeled after `git-sync`.
- Standardized CI release builds to a single Linux target (`ubuntu-24.04`) and removed Windows and distro-package jobs for now.
- Updated internal workspace `path` dependencies to include explicit version requirements for packaging compatibility.
- Switched editor asset validation to consume schema definitions from `toki-schemas`.

### Fixed
- Fixed `cargo release` workspace packaging by moving build scripts into package-local `build.rs` files.
- Fixed release configuration mismatches for branch policy, changelog replacement paths, and `0.0.x` version/tag flow.
- Fixed package-crate CI failures by using workspace packaging (`cargo package --locked --workspace`) instead of per-crate packaging.
- Fixed workspace clippy warnings in runtime/editor code and test assertions.
- Fixed packaged `toki-editor` schema include failures by resolving schemas from package-local crate assets.

## [0.0.11] - 2026-03-08

### Added
- Added a project `Justfile` with core workflow targets for build, run, lint, format, tests, and LLM/developer flows.
- Added broader unit-test coverage around editor entity placement and interaction behavior.
- Added inspector-driven property editing for selected scene entities.

### Changed
- Refactored editor UI architecture by splitting monolithic panel logic into focused interaction modules (`camera`, `placement`, `selection`).
- Transitioned editor interaction model to intuitive click-select plus drag-to-move behavior.
- Refactored entity creation paths to use definition-driven spawning consistently (removed factory-style divergence).
- Moved runtime audio state out of `Entity` into dedicated audio-component storage managed by `EntityManager`.
- Updated README and developer workflow guidance to reflect current command usage.

### Fixed
- Fixed active scene loading timing so scene content renders correctly after project open.
- Fixed runtime/entity rendering edge cases related to scene update ordering and viewport refresh.
- Fixed entity drag behavior to hide the original entity while moving and keep placement state until valid drop.
- Fixed hardcoded entity-definition mapping fallbacks in selection/move flows.
- Fixed viewport map-context regression where drag operations could revert to a different scene map.

## [0.0.10] - 2025-08-31

### Added
- Added centralized scene management support integrated with editor/runtime flows.
- Added project-wide asset management with entity loading integration.
- Added JSON schema support and validation flow for scenes, entities, atlases, and tilemaps.
- Added additional unit tests for new editor/core behavior.

### Changed
- Refactored shared systems into `toki-core` (including resource-management-related pieces and common utilities).
- Improved editor project-management and scene persistence workflow.

### Fixed
- Fixed scene save/load integration issues and editor scene visibility after loading.
- Fixed editor inspection and scene-entity integration behavior.
- Fixed clippy issues in touched modules.

## [0.0.9] - 2025-08-30

### Added
- Added initial editor foundation with panel/layout-driven UI and project/config handling.
- Added logging panel/workflow integration in the editor.
- Added viewport camera support and keyboard-layout-friendly input handling.

### Changed
- Reworked scene/map handling in editor workflows.
- Refined viewport rendering integration and nearest-neighbor behavior for pixel-art clarity.
- Improved editor performance and reduced logging noise in interactive loops.

### Fixed
- Fixed viewport texture presentation and rendering-path issues.
- Fixed continuous redraw issue that caused excessive CPU usage.

## [0.0.8] - 2025-08-26

### Added
- Added game-state serialization/save-load support.
- Added unit tests for serialization and persistence behavior.
- Added simple NPC AI support for multi-entity save/load verification.

### Changed
- Reworked audio event behavior to be state- and distance-driven.
- Refactored rendering and naming/layout organization for clearer module intent.

## [0.0.7] - 2025-08-25

### Changed
- Updated background music playback to stream instead of preloading.
- Improved audio effect handling and adjusted related runtime behavior.

### Fixed
- Fixed API/test-suite integration breakages introduced by prior audio changes.
- Fixed pipeline/dependency issues impacting CI stability.

### Tests
- Added additional tests around event and render behavior.

## [0.0.6] - 2025-08-24

### Added
- Added dedicated sound-system support in runtime flows.

## [0.0.5] - 2025-08-24

### Added
- Added initial audio engine support with background music loading.
- Added audio asset scaffolding for project/runtime use.
- Added animation-state support for player idle/walk behavior.

### Changed
- Streamlined sprite-atlas rendering integration.
- Replaced string-based clip lookup with enum-based animation-state handling.

## [0.0.4] - 2025-08-24

### Added
- Added tile-based collision detection integration.
- Added collision-box debug visualization support.
- Added advanced animation system with state-management-based clips.
- Added improved map assets for collision/animation testing.

### Changed
- Updated tests for collision-system integration.

## [0.0.3] - 2025-08-19

### Added
- Added entity-management system and integrated it with runtime state.
- Added/updated test exports and CI-related test support updates.

### Changed
- Moved runtime logic away from legacy sprite-only handling toward entity-driven systems.
- Improved integer-based positioning consistency.
- Updated README for new architecture/runtime behavior.

### Fixed
- Fixed movement and integration bugs during entity-system migration.
- Fixed test runner configuration issues.

## [0.0.2] - 2025-08-18

### Added
- Added frustum-culling support for tilemap rendering.
- Added performance statistics (FPS/frame-time and extended metrics).
- Added broader unit-test coverage across newly extracted core systems.

### Changed
- Refactored GPU/pipeline module organization.
- Refactored app architecture into clearer platform/render/timing/game subsystems.
- Refactored resource and camera usage patterns for cleaner separation.

### Fixed
- Fixed pixel-perfect integer coordinate handling for rendering.
- Fixed tilemap renderer edge coverage behavior.
- Updated timing behavior and recommendations for stability.

## [0.0.1] - 2025-08-18

### Added
- Initial workspace setup with core/render/runtime crates and baseline project configuration.
- First WGPU window/render path and initial sprite drawing.
- Basic sprite movement, tick-based update loop, and early animation support.
- Atlas/tilemap loading, map rendering, and JSON schema scaffolding for map assets.
- Camera follow/clamping behavior and large-map test assets.
- CI bootstrap (`rust.yml`) and initial unit-test coverage for core modules.
- README/license/docs baseline and asset handling groundwork (`git-lfs`).

### Changed
- Multiple early refactors splitting rendering and app logic into cleaner modules.
- Moved projection calculation into `toki-core` math utilities.

### Fixed
- Fixed sprite aspect ratio/projection correctness issues.
- Fixed camera/map-bound movement and projection distortion on resize.
- Improved tilemap upload strategy and window/surface resize handling.

[Unreleased]: https://github.com/daredem0/ToKi/compare/0.0.11...HEAD
[0.0.11]: https://github.com/daredem0/ToKi/compare/0.0.10...0.0.11
[0.0.10]: https://github.com/daredem0/ToKi/compare/0.0.9...0.0.10
[0.0.9]: https://github.com/daredem0/ToKi/compare/0.0.8...0.0.9
[0.0.8]: https://github.com/daredem0/ToKi/compare/0.0.7...0.0.8
[0.0.7]: https://github.com/daredem0/ToKi/compare/0.0.6...0.0.7
[0.0.6]: https://github.com/daredem0/ToKi/compare/0.0.5...0.0.6
[0.0.5]: https://github.com/daredem0/ToKi/compare/0.0.4...0.0.5
[0.0.4]: https://github.com/daredem0/ToKi/compare/0.0.3...0.0.4
[0.0.3]: https://github.com/daredem0/ToKi/compare/0.0.2...0.0.3
[0.0.2]: https://github.com/daredem0/ToKi/compare/0.0.1...0.0.2
[0.0.1]: https://github.com/daredem0/ToKi/releases/tag/0.0.1
