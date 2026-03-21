# 🎮 ToKi — Top-down Kit for Game Boy–Style Games

[![CI](https://github.com/daredem0/toki/actions/workflows/rust.yml/badge.svg)](https://github.com/daredem0/toki/actions/workflows/rust.yml)
[![Coverage](https://codecov.io/gh/daredem0/toki/graph/badge.svg?branch=main)](https://codecov.io/gh/daredem0/toki)
[![Docs](https://img.shields.io/badge/docs-github%20pages-2ea44f?logo=github)](https://daredem0.github.io/toki/)
[![Release](https://img.shields.io/github/v/release/daredem0/toki)](https://github.com/daredem0/toki/releases)
[![License](https://img.shields.io/badge/license-MPL--2.0%20libs%20%7C%20community%2Fcommercial%20apps-blue)](./README.md#license)
[![Rust Edition](https://img.shields.io/badge/rust-2021%20edition-black?logo=rust)](https://www.rust-lang.org/)

<p align="center">
  <img src="./assets/TokiLogo.png" alt="ToKi Logo" width="320" />
</p>

**ToKi** is a lightweight 2D game engine and editor for Game Boy-style top-down games.  
It is aimed at small, self-contained pixel-art projects that want an integrated workflow for gameplay code, authored assets, scene editing, map editing, runtime UI, and export.

The project is built as a modular Rust workspace with a shared core, a renderer, a runtime application, and a GUI-first editor. The overall direction is an engine that provides reusable building blocks instead of hardcoded game-specific features, while still staying focused on retro-style top-down games rather than trying to be a fully general-purpose engine.

**Status:** ToKi is still a work in progress. The core architecture and many major systems are in place, but the engine is still evolving and some areas are incomplete, being actively refactored, or not yet stabilized.

---

## ✨ Current Capabilities

- Modular workspace split into `toki-core`, `toki-render`, `toki-runtime`, `toki-editor`, and `toki-schemas`
- Pixel-art runtime with configurable resolution, zoom, timing mode, and optional frame limiting
- Animated sprite-atlas rendering and static object-sheet rendering
- Tilemap rendering with chunk caching and camera-follow support
- Scene/entity system with JSON-based project assets
- Projectile, pickup, and minimal inventory runtime support
- Visual editor with scene hierarchy, map editor, entity placement, inspector-driven asset editing, and runtime menu editor
- Shared runtime/editor menu and dialog composition pipeline
- Runtime pause menu, inventory view, confirmation dialogs, and clean exit actions
- Project-scoped font discovery from `assets/fonts`
- Project validation against JSON schemas
- Hybrid bundle export with `.pak` packaging and generated `runtime_config.json`

---

## 📁 Workspace Layout

```bash
toki/
├── crates/
│   ├── toki-core     # Shared game logic, asset/runtime models, UI composition, rules
│   ├── toki-schemas  # Canonical JSON schemas for asset validation
│   ├── toki-render   # WGPU rendering backend and low-level GPU integration
│   ├── toki-runtime  # Runtime app shell, input loop, audio, loading, pack startup
│   └── toki-editor   # GUI editor, viewport tools, inspectors, project workflow
├── example_project/  # Sample ToKi project used for manual/runtime testing
├── docs/             # Design and architecture documentation
└── assets/           # Workspace assets such as logo and documentation images
```

## 🧪 Running & Testing
### ▶️ Using `just` (recommended)
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

### ✅ Quality & Important Targets
```bash
just fmt-check
just clippy
just quality-docs
just quality-licenses-check
just quality-licenses-generate
just important
just llm
```

### 📊 Code Coverage
```bash
just install-llvm-cov
just coverage-open
```

### 📜 Dependency License Hygiene
```bash
just install-cargo-deny
just install-cargo-about
just quality-licenses-check
just quality-licenses-generate
```

### 🚀 Release Workflow (`cargo release`)
```bash
just install-cargo-release
just release-dry-run 0.0.13
just release-execute 0.0.13
```

Release behavior:
- Uses a shared workspace version for all crates.
- Allows releases from `main` and `develop`.
- Creates Git tags like `v0.0.13`.
- Expects `CHANGELOG.md` to be prepared before running release commands.
- Does not publish crates, but does push release commit/tag automatically when executed.

### ▶️ Direct Cargo Commands (equivalent)
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
cargo release 0.0.13 --workspace --no-publish
cargo release 0.0.13 --workspace --no-publish --execute
```

### 🎮 Editor Features
- **Interactive Viewport**: Mouse drag camera controls with configurable speed
- **Asset Management**: Automatic discovery of scenes, entities, tilemaps, atlases, object sheets, audio, and entity definitions
- **Entity Palette**: Drag-and-drop entity placement from definitions
- **Scene Hierarchy**: Visual scene management with scene entities, scene items, and optional runtime entities
- **Map Editor**: Tile painting, object placement, and viewport-based editing
- **Menu Editor**: Visual menu preview with inspector-driven editing for screens, entries, and dialogs
- **Project Export**: Hybrid bundle export with runtime config and `.pak` payload generation
- **Asset Validation**: Edit → "Validate Project Assets" for schema compliance checking

### ⌨️ Runtime Hotkeys
- `W` / `A` / `S` / `D`: Move player
- `Space`: Trigger primary action
- `Escape`: Open or close the runtime menu / dialog flow
- `F4`: Toggle collision debug rendering
- `F3`: Toggle in-window performance HUD text
- `F7`: Toggle console performance log output
- `F5`: Save game state to `savegame.json`
- `F6`: Load game state from `savegame.json`

### 📚 Documentation
- `just quality-docs` builds workspace rustdoc with Mermaid support.
- The `toki_core` rustdoc landing page includes both `README.md` and `docs/SDD_SAD.md`.

## Committing
Commit Message Shape Rules

1. First line format: `<Prefix>: Brief summary` (no trailing period).
2. Allowed prefixes: Add:, Change:, Fix:, Refactor:, Doc:, chore.
3. Leave exactly one blank line after the first line.
4. Body uses dash bullets ("- "), one change per line, no extra blank lines between bullets.
5. Keep bullets short and parallel in structure; wrap only if needed and indent continuation lines.
6. Use bullets to state what changed and why; avoid long prose paragraphs.
7. Only use more than 3 bullets for very large commits

Example:
```text
Change: Add tile trigger editing and viewport cursor readout

- Add `OnTileEnter` and `OnTileExit` rule editing in the core and inspector
- Migrate the example project rule graph and scene data to the new trigger shape
- Show the scene viewport cursor position in the toolbar with a `P/T` tile toggle
- Remove the dead selected-entity viewport stub and keep cursor state persistent
```

## License

This workspace currently uses a mixed licensing model:

- `toki-core`, `toki-schemas`, and `toki-render` are licensed under **MPL-2.0**
- `toki-runtime` and `toki-editor` use the application-layer license in `LICENSE.md`

See each crate's `Cargo.toml` for the authoritative package license.

The application-layer packages use this model:

- **Non-commercial and non-monetized use** — see `LICENSE.md`
- **Separate written commercial agreement required** — see `LICENSE-COMMERCIAL.md` for commercial or monetized use

`LICENSE.md` is the canonical and Cargo-facing license file for `toki-runtime` and `toki-editor`.

### Logo Rights

The ToKi logo file at `assets/TokiLogo.png` is **not** covered by the source-code licenses above.  
All rights to the ToKi logo are reserved.
