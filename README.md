# 🎮 ToKi — Top-down Kit for Game Boy–Style Games

[![CI](https://github.com/daredem0/toki/actions/workflows/rust.yml/badge.svg)](https://github.com/daredem0/toki/actions/workflows/rust.yml)
[![Coverage](https://codecov.io/gh/daredem0/toki/graph/badge.svg?branch=main)](https://codecov.io/gh/daredem0/toki)
[![Docs](https://img.shields.io/badge/docs-github%20pages-2ea44f?logo=github)](https://daredem0.github.io/toki/)
[![Release](https://img.shields.io/github/v/release/daredem0/toki)](https://github.com/daredem0/toki/releases)
[![License](https://img.shields.io/github/license/daredem0/toki)](./LICENSE)
[![Rust Edition](https://img.shields.io/badge/rust-2024%20edition-black?logo=rust)](https://www.rust-lang.org/)

**ToKi** is a lightweight, pixel-perfect 2D game engine and editor inspired by the visual and design constraints of the original Nintendo Game Boy.  
It provides a modular toolkit for making retro-style games with clean pixel graphics, tilemaps, animations, and visual scripting — all self-contained and offline.

---

## ✨ Features (WIP)

-  Game Boy–style sprite rendering (160×144 resolution)
-  Animation system with frame timing + loop control
-  Pixel-perfect camera & scaling (integer nearest-neighbor)
-  Modular architecture: core, render, editor, runtime
- CLI-free, GUI-focused editor with interactive viewport
- Tilemap + chunked rendering engine
- Entity system with JSON-based definitions
- Asset management with automatic project scanning
- JSON schema validation for all asset types
- Scene management with serialization support
- Visual rules editor (no-code logic) - planned
- One-click export to native binary - planned

---

## 📁 Crate Structure

```bash
toki/
├── crates/
│   ├── toki-core     # Data models, math, animation, entity system, scene management
│   ├── toki-schemas  # Canonical JSON schemas for asset validation
│   ├── toki-render   # WGPU-based renderer with scene rendering support  
│   ├── toki-runtime  # Game loop & logic runtime
│   └── toki-editor   # Editor UI with interactive viewport and asset management
├── example_project/  # Sample project demonstrating asset structure
└── assets/           # Sprites, atlases, maps (JSON/PNG)
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
just release-dry-run 0.0.12
just release-execute 0.0.12
```

Release behavior:
- Uses a shared workspace version for all crates.
- Allows releases from `main` and `develop`.
- Creates numeric tags like `0.0.12` (matching existing repository tags).
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
cargo release 0.0.12 --workspace --no-publish
cargo release 0.0.12 --workspace --no-publish --execute
```

### 🎮 Editor Features
- **Interactive Viewport**: Mouse drag camera controls with configurable speed
- **Asset Management**: Automatic discovery of scenes, entities, atlases, and maps
- **Entity Palette**: Drag-and-drop entity placement from definitions
- **Scene Hierarchy**: Visual scene management with entity organization
- **Asset Validation**: Edit → "Validate Project Assets" for schema compliance checking

### 📚 Documentation
- `just quality-docs` builds workspace rustdoc with Mermaid support.
- The `toki_core` rustdoc landing page includes both `README.md` and `docs/SDD_SAD.md`.

## Committing
Commit Message Shape Rules

1. First line format: `&lt;Prefix&gt;: Brief summary` (no trailing period).
2. Allowed prefixes: Add:, Change:, Fix:, Refactor:, Doc:, chore.
3. Leave exactly one blank line after the first line.
4. Body uses dash bullets ("- "), one change per line, no extra blank lines between bullets.
5. Keep bullets short and parallel in structure; wrap only if needed and indent continuation lines.
6. Use bullets to state what changed and why; avoid long prose paragraphs.
7. Only use more than 3 bullets for very large commits

## License

This project is dual‑licensed under either:

- **GPL‑3.0‑or‑later** — see `LICENSE`; or
- **Toki Commercial License v1.0** — see `LICENSE-COMMERCIAL.md`.

Choose the option that fits your needs. To use this software in a proprietary product without GPL copyleft obligations, contact me to purchase a commercial license.
