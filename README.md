[![pipeline](https://github.com/daredem0/ToKi/actions/workflows/rust.yml/badge.svg)](https://github.com/daredem0/ToKi/actions/workflows/rust.yml)

# 🎮 ToKi — Top-down Kit for Game Boy–Style Games

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
│   ├── toki-render   # WGPU-based renderer with scene rendering support  
│   ├── toki-runtime  # Game loop & logic runtime
│   └── toki-editor   # Editor UI with interactive viewport and asset management
├── schemas/          # JSON schemas for asset validation
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
just important
just llm
```

### 📊 Code Coverage
```bash
just install-llvm-cov
just coverage-open
```

### ▶️ Direct Cargo Commands (equivalent)
```bash
cargo build
cargo run -p toki-editor
cargo run -p toki-runtime
cargo test --workspace
cargo install cargo-llvm-cov
cargo llvm-cov -p toki-core --open
```

### 🎮 Editor Features
- **Interactive Viewport**: Mouse drag camera controls with configurable speed
- **Asset Management**: Automatic discovery of scenes, entities, atlases, and maps
- **Entity Palette**: Drag-and-drop entity placement from definitions
- **Scene Hierarchy**: Visual scene management with entity organization
- **Asset Validation**: Edit → "Validate Project Assets" for schema compliance checking

## License

This project is dual‑licensed under either:

- **GPL‑3.0‑or‑later** — see `LICENSE`; or
- **Toki Commercial License v1.0** — see `LICENSE-COMMERCIAL.md`.

Choose the option that fits your needs. To use this software in a proprietary product without GPL copyleft obligations, contact me to purchase a commercial license.
