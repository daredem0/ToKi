# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-03-22

### Added
- Added Magic Erase tool in Sprite Editor for tile-local background cleanup using flood-fill removal of connected same-color pixels.
- Added pixel highlight under cursor in Sprite Editor for precise editing feedback.
- Added F11 keyboard shortcut to toggle borderless fullscreen mode in the editor.
- Added outline application function in Sprite Editor to add outlines to sprites.
- Added optional ground shadow rendering with configurable oval shape for entities.

### Changed
- Structured SpriteEditor Tools in a grid.
- Changed editor tab bar to be scrollable to handle many open tabs.
- Changed ground shadow shape from circular to oval for more natural appearance.
- Refactored entity data structure from legacy monolithic module to component-based format with explicit animation, collision, AI, interaction, and movement components.
- Refactored AI system with `AiContext` and behavior handlers (Chase, Run, Wander) using a strategy pattern for cleaner flow.
- Refactored inspector UI and rule graph editor into modular logic with extracted shared helpers.
- Refactored AI and rules systems into focused submodules (behaviors, movement, context, events, commands, transitions, targeting, evaluation).
- Refactored editor/runtime integration with clearer data flow for scene, project, and graph state handling.
- Refactored runtime app into `WorldFramePresenter`, `SceneRuntimeCoordinator`, `StartupCoordinator`, and `StartupBundle` components.
- Refactored runtime services by extracting dedicated modules for input, AI, and reactive rules.
- Refactored stat effect handling into dedicated service with scene transition planner.
- Refactored asset discovery and loading to use centralized shared helpers across editor and runtime.
- Refactored `GameState` construction to use centralized project content initialization.
- Refactored editor viewport math and interaction context into shared extracted modules.
- Removed legacy entity module, rule-graph key migration, dead sprite-editor scaffolding, and broad `allow(dead_code)` annotations.
- Removed legacy and unused code paths across project assets, editor state, and viewport interaction.

### Fixed
- Fixed Sprite Editor rectangle selection to correctly include both start and end pixels.
- Fixed clippy warnings across the workspace.
- Fixed code formatting across affected modules.

### Docs
- Clarified entity attribute groups and runtime field purpose in documentation.

## [0.2.0] - 2026-03-21

### Added
- Added a full-featured Sprite Editor with pixel-level editing, drawing tools (brush, eraser, fill, line, rectangle), color picker, undo/redo history, and keyboard shortcuts.
- Added dual-canvas support in the Sprite Editor for side-by-side sprite sheet editing with copy/paste between canvases.
- Added sprite sheet extension and import/merge capabilities to combine existing sprite assets.
- Added canvas resize tools (expand, shrink, crop to content) in the Sprite Editor.
- Added an Animation Editor with sprite atlas preview, clip management, frame duration editing, and live animation playback preview.
- Added draggable panel dividers in the Animation Editor for customizable layout.
- Added zoom controls (+/- keys) in the Animation Editor and scroll-wheel zoom across editor viewports.
- Added an Entity Editor panel foundation with entity browsing, category filtering, and property editing infrastructure.
- Added entity definition authoring with sprite atlas dropdown selection and SFX audio dropdown selection.
- Added spawn point authoring in scenes with draggable viewport placement and facing direction configuration.
- Added scene deletion support in the editor.
- Added scene-to-scene transitions with fade-to-black effects and music crossfade.
- Added player state preservation across scene transitions (health, inventory, stats transfer; transient state resets).
- Added tile-based rule triggers (`OnTileEnter`, `OnTileExit`) with viewport cursor position readout.
- Added context-aware rule triggers exposing collision pairs, damage attacker/victim, and death context.
- Added rule conditions for querying entity state, stats, and game context.
- Added rule actions for modifying entity stats, setting velocity, and teleporting entities.
- Added runtime validation for out-of-bound rule references.
- Added AI behavior system with `Wander`, `Chase`, `Run`, and `RunAndMultiply` behaviors.
- Added authored AI configuration in entity definitions with detection radius and behavior-specific parameters.
- Added `RunAndMultiply` AI that flees from threats, seeks allies, and spawns new entities on collision with cooldown.

### Changed
- Changed rule editor to use streamlined flat editing alongside the graph view.
- Changed teleport action to use tile-based coordinates instead of pixel coordinates.
- Changed player entity model to be unique per scene and connected to spawn points.
- Changed scene hierarchy to support cleaner spawn point and player entity visualization.
- Changed animation editor to sync atlas metadata on save, preserving tile names from entity definitions.
- Changed sprite editor to auto-shrink pasted content that exceeds cell boundaries.
- Refactored editor UI to use `EnumIter` pattern for cleaner enum iteration.

### Fixed
- Fixed background music playback and added smooth transition effects between tracks.
- Fixed spawn point interaction in viewport (draggable and clickable).
- Fixed rendering issues after player entity refactoring.
- Fixed chessboard and canvas drag alignment in sprite editor.
- Fixed clippy warnings across the workspace.

## [0.1.1] - 2026-03-19

### Added
- Added project-wide health and damage stat support, including optional attack animations driven by primary-action input and sample player attack clips.
- Added authored projectile support with runtime spawning, movement, collision, damage, lifetime expiry, and object-sheet rendering.
- Added collectible world-item pickups with minimal inventory stacking and static object-sheet-backed entity rendering.
- Added a first runtime menu/UI stack with pause menus, inventory views, confirmation dialogs, generic UI actions/commands, and clean runtime exit handling.
- Added a visual Menu Editor tab in the editor with preview-based menu/dialog authoring and inspector-driven configuration.
- Added extensive runtime menu appearance controls, including font selection, colors, opacity, border style, spacing, footer text, and viewport-relative sizing.
- Added shared project-font discovery from `assets/fonts` for editor preview and runtime menu rendering.
- Added optional frame limiting when vsync is disabled and introduced fixed-vs-delta timing mode selection at the project/runtime level.
- Added configurable viewport resolution, zoom, tile-size selection for new maps, and arbitrary viewport sizing with a 160×144 default.
- Added reusable workspace infrastructure including a generic asset cache, shared project-runtime config model, shared project-asset resolution helpers, and a shared sprite-render request pipeline.

### Changed
- Refactored `GameState` into focused `game/` submodules for scene, input, movement, animation, combat, rules, inventory, and render-query responsibilities.
- Refactored the runtime app shell into dedicated bootstrap, lifecycle, splash, and tick modules and replaced the previous render-backend wrapper with a proper trait abstraction.
- Refactored the editor app into grouped subsystems and split viewport, hierarchy, inspector, scene-graph, and map-editor code into focused modules.
- Changed runtime and editor world-sprite rendering to use one shared sprite-render extraction/resolution model instead of parallel implementations.
- Changed runtime/editor menu rendering to use shared UI composition and layout logic so preview and runtime behavior stay aligned.
- Changed asset handling to share more implementation between runtime and editor, including normalized asset names, shared scene-path utilities, and shared audio/sprite metadata classification.
- Changed camera and viewport handling to use shared camera/viewport calculation paths and configurable runtime/editor display parameters.
- Changed movement to support sub-pixel accumulation and intent-driven animation selection instead of relying on older hardcoded movement constants.
- Changed scene hierarchy and asset-palette ergonomics with clearer grouping for scene entities/items, collapsible sections, runtime-entity visibility toggles, and cleaner icon usage.
- Changed project creation flow to an explicit editor-owned modal with template selection, naming, and folder selection instead of silently creating nested default paths.

### Fixed
- Fixed menu preview/runtime mismatches in layout, font handling, border/background rendering, and screen centering.
- Fixed a runtime menu rendering crash caused by using the wrong rectangle-rendering path and introduced a dedicated runtime UI rectangle layer.
- Fixed font preview crashes in the editor by unifying editor/runtime font handling and providing shared font-family resolution.
- Fixed project-creation behavior that could create accidental nested `NewProject/NewProject` trees.
- Fixed stale scene-hierarchy and menu-editor interaction issues, including widget-ID collisions in dialog entry editing.

### Docs
- Updated `README.md` to reflect the current product surface, workspace structure, editor/runtime capabilities, and work-in-progress status.
- Updated `docs/SDD_SAD.md` to match the current architecture, including shared menu/UI composition, shared sprite-render flow, current runtime/editor decomposition, and the remaining known seams.

### Tests
- Added broad regression coverage for the new health/damage, projectile, pickup/inventory, menu/dialog, timing, frame-limiter, shared asset/config, and shared sprite-render workflows.
- Expanded runtime, editor, render, and core test coverage around the refactored subsystem boundaries to lock in the new architecture.

## [0.1.0] - 2026-03-15

### Added
- Added planned runtime asset loading with hot-asset caching and pack-path support for project-backed and exported-game startup flows.
- Added a top-down starter template and example project content to give ToKi a reusable baseline for top-down games.
- Added directional character animation support across core/runtime/editor, including multi-atlas sprite loading and horizontal sprite mirroring for left-facing movement.
- Added solid-entity collision so movement blocking now works against other solid actors instead of only solid terrain.
- Added configurable AI behavior selection with a first `Wander` mode exposed in the editor inspector.
- Added explicit movement profiles with `PlayerWASD` as the first implemented scheme and support for multiple input-controlled entities moving from the same profile.
- Added scene-level `control_role` semantics so player-character identity is authored per placed scene entity instead of being hardcoded in shared definitions.
- Added generic `category`-driven authoring semantics and updated the editor palette to group definitions by reusable categories such as `human` and `creature`.
- Added editor support for entity-definition and scene-level audio controls, including movement sound, footstep distance, trigger mode, and hearing radius.
- Added a right-side Project panel in the editor for project-wide settings and introduced an audio mixer with master, music, movement, and collision channel sliders.
- Added derived-version presentation in editor and runtime, including runtime startup logging and splash-screen version display.
- Added an independent Map Editor tab with in-memory map drafts, explicit save, tile brush/fill/pick tools, brush previews, responsive viewport sizing, fine-grained zoom, and undo/redo for map edits.
- Added typed object-sheet assets for placeable map sprites and first-pass map-object placement, selection, dragging, visibility, solidity, and deletion in the map editor.
- Added runtime rendering for map-owned object-sheet instances so placed map objects now appear in-game.

### Changed
- Changed runtime/editor sprite loading to discover project atlases dynamically instead of depending on a single hardcoded creature atlas.
- Changed authored player/NPC semantics to a cleaner split between `category`, `control_role`, `movement_profile`, and `ai_behavior`.
- Renamed the internal runtime entity enum from `EntityType` to `EntityKind` to match the new authoring model.
- Changed movement sound handling from input-coupled playback to generic movement-driven playback with configurable trigger policies (`distance` or `animation_loop`).
- Changed scene-level audio settings so placed entities can override definition defaults for locomotion/collision behavior within a scene.
- Changed text anchoring and splash layout so derived version strings center correctly and fit narrow runtime views.
- Changed the map editor workflow from scene-coupled map loading to independent asset editing with its own viewport state and save flow.
- Changed map-object authoring so placed objects now persist their size, visibility, and solidity as part of the tilemap asset.
- Changed newly placed map objects to default to `solid = true`.

### Fixed
- Fixed editor/runtime multi-texture rendering regressions so mixed atlases such as `players.json`, `creatures.json`, and object sheets can coexist without overwriting each other.
- Fixed editor viewport startup redraw/projection issues that previously hid sprites until the camera was moved.
- Fixed runtime sprite rendering for texture-specific pipelines by propagating projection state to all atlas batches.
- Fixed map save behavior for unsaved draft maps so painted changes are written from the live viewport state instead of being reset to the initial fill tile.
- Fixed runtime resource loading to ignore object-sheet JSON files when building the sprite atlas registry.
- Fixed splash branding/version overlap and centering issues in the runtime startup screen.
- Fixed movement audio so non-player movement sources such as wander AI and rule-driven velocity now emit sound correctly.
- Fixed editor logging spam from per-frame missing-directory messages in the map panel.

### Tests
- Added broad regression coverage for the new movement-profile, control-role, audio-mixer, object-sheet, map-editor, and runtime resource-loading workflows.
- Added schema tests for the new object-sheet and map-object formats.
- Added persistence and collision tests for painted maps and solid map objects.
- Expanded editor interaction tests around map painting, object placement, object selection/dragging/deletion, and map-editor undo/redo behavior.

## [0.0.14] - 2026-03-13

### Added
- Added the visual rules baseline across runtime/editor/schema with scene-authored rule loading and inspector authoring support.
- Added rule triggers `OnCollision`, `OnTrigger`, and `OnPlayerMove` with end-to-end runtime/editor/schema support.
- Added rule actions `PlayMusic`, `Spawn`, and `DestroySelf` with inspector authoring support.
- Added runtime rule conditions beyond `Always` (target existence, key-held state, and simple entity-active checks) with inspector authoring support.
- Added an editor `Play Scene` workflow that launches runtime for the currently active scene and map.
- Added runtime startup argument support for project/scene/map overrides so editor Play Scene mode can boot into the selected content.
- Added a tabbed center workspace in `toki-editor` (`Scene Viewport`, `Scene Graph`, `Scene Rules`) and introduced a graph-backed `RuleGraph` model.
- Added scene graph authoring operations for adding trigger/condition/action nodes, editing node payloads in the inspector, and connecting/disconnecting nodes.
- Added direction-aware rule graph editing affordances in inspector (`Connect To` for outgoing and `Connect From` for incoming).

### Changed
- Changed runtime rule execution to deterministic buffered command processing with stable ordering semantics.
- Changed `SwitchScene` behavior from placeholder handling to deterministic end-of-tick scene switching with state/map consistency safeguards.
- Changed scene graph rendering and persistence to use `RuleSet <-> RuleGraph` conversion so inspector and graph authoring stay serialization-compatible.
- Changed graph visuals and ergonomics with edge-based auto-layout spacing, clearer node labeling, direction arrows, and improved zoom/pan behavior.
- Changed editor startup flow to auto-open the last configured project path.
- Changed runtime audio state handling to a component-driven approach as part of audio cleanup.

### Fixed
- Fixed runtime initialization when launched from editor play mode by ensuring GPU/resource setup uses the active project texture context.
- Fixed Play Scene behavior that could fall back to hardcoded runtime content instead of the active editor scene/map.
- Fixed standalone condition/action nodes so they stay detached until explicitly connected.
- Fixed graph connection behavior so adding a new edge no longer removes existing outgoing edges.
- Fixed cross-chain connection regressions that could cause node position jumps.
- Fixed scene rule graph persistence by saving/loading graph drafts and connection edges through project metadata.
- Fixed graph spacing and layout behavior so spacing is applied relative to node edges (not centers).

### Tests
- Added focused visual-rules test coverage across stepwise implementation milestones (baseline, deterministic ordering, trigger emissions, and authoring behavior).
- Raised unit-test coverage across stable `toki-core`/`toki-render` paths and added additional runtime unit tests.
- Added rule-graph tests for deterministic roundtrip parity, invalid graph rejection, connection safety, and graph edit operation stability.

## [0.0.13] - 2026-03-08

### Added
- Added a current-state combined software design and architecture document in `docs/SDD_SAD.md`.
- Added a dedicated `docs/legal/` location for auxiliary legal guidance such as the editor plugin compatibility notice.

### Changed
- Integrated the project `README.md` and `docs/SDD_SAD.md` into workspace rustdoc so generated docs expose both the product overview and architecture guidance.
- Updated the local docs workflow and CI docs job to build Mermaid-enabled workspace rustdoc output.
- Switched future release tags to the `v0.0.x` style to align ToKi's release flow with `git-sync`.
- Simplified repository licensing layout by consolidating application-layer terms into `LICENSE.md` and updating crate metadata to match the mixed MPL/community-commercial model.
- Reclassified `toki-render` and `toki-schemas` as `MPL-2.0` crates to match the intended product-vs-library license split.

## [0.0.12] - 2026-03-08

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

[Unreleased]: https://github.com/daredem0/ToKi/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/daredem0/ToKi/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/daredem0/ToKi/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/daredem0/ToKi/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/daredem0/ToKi/compare/v0.0.14...v0.1.0
[0.0.14]: https://github.com/daredem0/ToKi/compare/v0.0.13...v0.0.14
[0.0.13]: https://github.com/daredem0/ToKi/compare/0.0.12...v0.0.13
[0.0.12]: https://github.com/daredem0/ToKi/compare/0.0.11...0.0.12
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
