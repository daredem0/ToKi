# ToKi — Top-down Kit for Game Boy–Style Games

[![CI](https://github.com/daredem0/toki/actions/workflows/ci.yml/badge.svg)](https://github.com/daredem0/toki/actions/workflows/rust.yml)
[![Coverage](https://codecov.io/gh/daredem0/toki/graph/badge.svg?branch=main)](https://codecov.io/gh/daredem0/toki)
[![Docs](https://img.shields.io/badge/docs-github%20pages-2ea44f?logo=github)](https://daredem0.github.io/toki/)
[![Release](https://img.shields.io/github/v/release/daredem0/toki)](https://github.com/daredem0/toki/releases)
[![License](https://img.shields.io/badge/license-MPL--2.0%20libs%20%7C%20ToKi%20License%20apps-blue)](./LICENSE.md)
[![Rust Edition](https://img.shields.io/badge/rust-2021%20edition-black?logo=rust)](https://www.rust-lang.org/)

<p align="center">
  <img src="./assets/TokiLogo.png" alt="ToKi Logo" width="320" />
</p>

**ToKi** is a fully data-driven 2D game engine and editor for Game Boy-style top-down games. Design your entire game — entities, scenes, rules, AI, maps, menus, dialogs, UI, and all gameplay logic — inside a visual editor, then export a ready-to-play build. **No programming required.**

The runtime is a generic interpreter: it reads JSON project data and a `.toki.pak` archive at startup and executes the game from that data alone. The same runtime binary runs any ToKi game without recompilation. Shipping a game is pairing editor-authored data with the pre-built runtime.

ToKi targets small, self-contained pixel-art projects that want an integrated workflow for asset authoring, scene editing, map editing, runtime UI, and export — all from one tool. It is built as a modular Rust workspace and stays focused on retro-style top-down games rather than being a general-purpose engine.

**Status:** ToKi is under active development. The core architecture and all major systems are in place and working. Some systems (lighting, tilemap layers, export pipeline) are still pending.

---

## Current Capabilities

### Engine & Runtime
- Modular workspace: `toki-core`, `toki-render`, `toki-runtime`, `toki-editor`, `toki-schemas`
- Pixel-art runtime with configurable resolution, zoom, and optional frame limiting
- Viewport modes: `AspectFit`, `IntegerScale`, `WindowFill` with live display-menu controls
- Runtime post-processing: Game Boy palette, ordered-dither quantization, tint, brightness/saturation, vignette
- Project-level palette system with palette files, indexed atlas color-mode metadata, and palette conversion tooling
- Animated sprite-atlas rendering and static object-sheet rendering with correct depth-sorted draw order
- Tilemap rendering with chunk caching and camera-follow support
- Grounding-based fake-depth sorting: entities and decorations sort by ground contact point, not sprite bounds
- Sprite drop shadows anchored at the grounded footprint
- FPS-independent movement via sub-pixel accumulators for smooth projectile and entity motion

### Entity & Scene System
- Unified entity type system: `Creature`, `Human`, `Item`, `Decoration` with capability-based optional components
- Optional capability components: `MovementComponent`, `AiComponent`, `InteractionComponent`, `CombatComponent`
- `EntityKind::Decoration` for static scenery (no movement/AI/combat overhead unless promoted)
- `EntityKind::Item` with `PickupDef` for first-class item/pickup semantics
- Animated decorations with idle-state animation clips
- AI behaviors: `Wander`, `Chase`, `Run`, `RunAndMultiply` with authored detection radius and behavior parameters
- Projectile, pickup, and inventory runtime support
- JSON-based project assets with schema validation

### Narrative & Gameplay Logic
- Rule graph and flat rule authoring: event-condition-action pipelines with visual graph editor
- Rule triggers: `OnTileEnter`, `OnTileExit`, entity collision, damage, death, dialog events
- Rule conditions: flag queries, entity state/stat queries, expression conditions
- Rule actions: entity stat modification, velocity, teleport, spawn, emit particles (preset), scene switch, audio, UI, save/load
- Expression language: arithmetic, comparisons, logical operators, `flags.*`/`self.*`/`target.*` variable paths, `min()`, `max()`, `random()`, `abs()`
- Dynamic action fields backed by expressions: damage/heal amounts, flag values, velocity, teleport targets, spawn positions
- Game flags: `Bool`, `Int`, `String` values, persisted across scene transitions
- Dialog system: standalone dialog assets, branching on flags, line/end node conditions, placement and styling controls
- Save/load: stable `SaveData` with scene, player state, flags, camera, and authored-entity persistence; 3+ slots with timestamp metadata; `SaveGame`/`LoadGame` rule actions and menu actions
- Scene transitions: fade with configurable duration, project-level defaults, authored per-`SwitchScene` override

### Editor
- **Scene Viewport**: unified spatial canvas for entity, decoration, item, and spawn-point placement, move, delete, and selection — single undo/redo history for all spatial operations
- **Toolbox panel**: kind-first placement surface (`Creatures`, `Humans`, `Items`, `Decorations`) — the left-side Assets list is browse/inspect-only
- **Map Editor**: tile-only (brush, fill, pick); decoration/object placement moved to the Scene Viewport
- **Inspector**: capability-driven — shows only the sections relevant to the selected entity's kind and component presence; promoted decorations keep movement/AI/combat editing surface
- **Sprite Editor**: full pixel-level editor with brush, eraser, fill, line, rectangle, ellipse, magic erase, color picker, outline tool, selection (drag, float, paste preview), symmetry, dithered painting, undo/redo, dual-canvas, canvas resize/crop, import/merge, palette conversion
- **Animation Editor**: atlas preview, clip management, frame duration editing, live playback preview
- **Entity Editor**: entity browsing, category filtering, property editing, sprite/audio dropdown selection
- **Dialog Editor**: standalone dialog asset authoring with node graph, branching conditions, and styling controls
- **UI Layout Editor**: visual canvas for authoring runtime HUD layouts with drag/resize, widget inspection, and shared editor/runtime scaling
- **Rule Graph Editor**: visual node-based rule authoring with expression validation and flag condition editing
- **Flag Manager**: project-level flag registry with list, create, rename, delete
- **Menu Editor**: visual menu preview with inspector-driven editing for screens, entries, submenus, and dialogs
- **Project Export**: hybrid bundle export with `.toki.pak` packaging and generated `runtime_config.json`
- **Asset Validation**: schema compliance checking for all project assets
- Per-frame LRU pipeline cache for textured sprites to reduce GPU state churn
- F11 borderless fullscreen toggle; scrollable tab bar

### Runtime UI
- Generic widget backbone: `Label`, `ProgressBar`, `GridContainer`, `Button`, `Slider`
- Anchored layout system with margin/padding; data binding via expression-based value paths
- `UiTheme` as the shared base; thin per-menu/dialog layout overrides
- HUD composition for persistent on-screen UI; rule actions: `ShowUi`, `HideUi`, `UpdateUiBinding`
- Project-wide UI event registry with editor validation for authored event identifiers
- Pause menu, confirmation dialogs, inventory view, and runtime settings overlays all rendered through the shared widget pipeline
- Runtime mouse hover/click interaction for menus, dialogs, submenus, and settings overlays

---

## Workspace Layout

```bash
toki/
├── crates/
│   ├── toki-core       # Shared game logic, asset/runtime models, UI composition, rules
│   ├── toki-schemas    # Canonical JSON schemas for asset validation
│   ├── toki-render     # WGPU rendering backend and low-level GPU integration
│   ├── toki-runtime    # Runtime app shell, input loop, audio, loading, pack startup
│   └── toki-editor     # GUI editor, viewport tools, inspectors, project workflow
├── example_project/    # Sample ToKi project used for manual/runtime testing
├── docs/               # Design and architecture documentation
└── assets/             # Workspace assets such as logo and documentation images
```

---

## Running & Testing

### Using `just` (recommended)
```bash
just help
just build
just run-editor
just run-runtime
just test
```

To run the runtime directly against the sample project:
```bash
cargo run -p toki-runtime -- --project example_project/NewProject --scene "Main Scene"
```

### Quality & Important Targets
```bash
just fmt-check
just clippy
just quality-docs
just quality-licenses-check
just quality-licenses-generate
just important
just llm
```

### Code Coverage
```bash
just install-llvm-cov
just coverage-open
```

### Dependency License Hygiene
```bash
just install-cargo-deny
just install-cargo-about
just quality-licenses-check
just quality-licenses-generate
```

### Release Workflow (`cargo release`)
```bash
just install-cargo-release
just release-dry-run 0.2.5
just release-execute 0.2.5
```

Release behavior:
- Uses a shared workspace version for all crates.
- Allows releases from `main` and `develop`.
- Creates Git tags like `v0.2.5`.
- Expects `CHANGELOG.md` to be prepared before running release commands.
- Does not publish crates, but does push the release commit/tag automatically when executed.

### Direct Cargo Commands (equivalent)
```bash
cargo build
cargo run -p toki-editor
cargo run -p toki-runtime
cargo test --workspace
cargo install cargo-llvm-cov
cargo llvm-cov -p toki-core --open
cargo install cargo-deny
cargo deny check licenses
cargo install cargo-about
cargo about generate --locked about.hbs > THIRD_PARTY_LICENSES.md
cargo install cargo-release
cargo release 0.2.5 --workspace --no-publish
cargo release 0.2.5 --workspace --no-publish --execute
```

---

## Runtime Hotkeys

- `W` / `A` / `S` / `D`: Move player
- `Space`: Trigger primary action
- `Escape`: Open or close the runtime menu / dialog flow
- `F3`: Toggle in-window performance HUD text
- `F4`: Toggle collision debug rendering
- `F7`: Toggle console performance log output

---

## Documentation

- `just quality-docs` builds workspace rustdoc with Mermaid support.
- The `toki_core` rustdoc landing page includes both `README.md` and `docs/SDD_SAD.md`.

---

## Committing

Commit Message Shape Rules:

1. First line format: `<Prefix>: Brief summary` (no trailing period).
2. Allowed prefixes: `Add:`, `Change:`, `Fix:`, `Refactor:`, `Doc:`, `chore:`.
3. Leave exactly one blank line after the first line.
4. Body uses dash bullets (`- `), one change per line, no extra blank lines between bullets.
5. Keep bullets short and parallel in structure; wrap only if needed and indent continuation lines.
6. Use bullets to state what changed and why; avoid long prose paragraphs.
7. Only use more than 3 bullets for very large commits.

Example:
```text
Change: Add tile trigger editing and viewport cursor readout

- Add `OnTileEnter` and `OnTileExit` rule editing in the core and inspector
- Migrate the example project rule graph and scene data to the new trigger shape
- Show the scene viewport cursor position in the toolbar with a `P/T` tile toggle
- Remove the dead selected-entity viewport stub and keep cursor state persistent
```

---

## Future Scope

Work is organized into phases. Phases 1 (Playable Narrative Game), 3.1 (In-Game UI Framework), 3.2 (Expression Language), 6 (Unified Scene Authoring), and 7 (Entity Capability Decomposition) are complete or largely complete. Active and upcoming work:

**Phase 2 — Rich World Presentation**
- Multiple tilemap layers (`ground`, `detail`, `above_entity`, `collision_only`)
- Auto-tiling (4-bit and 8-bit bitmask rules)
- Animated tiles with frame sequences
- Ambient lighting and point lights (optional, additive layer)
- Light occlusion and shadow casting
- Particle system with emitter presets

**Phase 3 (remaining) — Deeper Authoring Without Code**
- Advanced AI: patrol waypoints, guard behavior, line of sight, A* pathfinding
- Advanced animation: frame events, transition durations, animation layers, tween system
- Camera: smooth follow with dead zone, camera shake, bounds, scripted sequences
- Audio: spatial falloff, music layers/crossfades, sound variations, audio buses

**Phase 4 — Reliable Shipping**
- Asset packing into `.toki.pak`
- Standalone export for Linux, Windows, macOS
- Web export via wasm-pack + WebGPU
- CI packaging and artifact verification

**Phase 5 — Ecosystem**
- Networking / multiplayer
- Plugin and mod system
- Accessibility features
- GPU particle rendering, texture atlas auto-packing, profiler overlay

---

## License

ToKi uses a mixed licensing model. See [LICENSE.md](./LICENSE.md) for the quick-reference overview.

### Open components — MPL-2.0

- `toki-core`
- `toki-render`
- `toki-schemas`

These crates are licensed under the [Mozilla Public License 2.0](https://www.mozilla.org/en-US/MPL/2.0/).

### Application layer — ToKi License

- `toki-runtime`
- `toki-editor`

These components are governed by [LICENSE-TOKI.md](./LICENSE-TOKI.md).

**Non-commercial use is free.** This includes learning, hobby projects, school projects, game jams without monetization, and free game releases with no paid access or monetized features.

**Commercial use is permitted under LICENSE-TOKI.md without a separate written agreement**

If your game earns commercial revenue, read [LICENSE-TOKI.md](./LICENSE-TOKI.md) for reporting, payment, attribution, and release-notice requirements.

See each crate's `Cargo.toml` for the authoritative package license field.

### Logo Rights

The ToKi logo at `assets/TokiLogo.png` is **not** covered by the source-code licenses above. All rights to the ToKi logo are reserved.
