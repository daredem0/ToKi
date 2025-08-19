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
- CLI-free, GUI-focused editor planned
- Tilemap + chunked rendering engine
- Visual rules editor (no-code logic)
- One-click export to native binary

---

## 📁 Crate Structure

```bash
toki/
├── crates/
│   ├── toki-core     # Data models, math, animation, image parsing
│   ├── toki-render   # WGPU-based renderer
│   ├── toki-runtime  # Game loop & logic runtime
│   └── toki-editor   # Editor UI (egui/winit/ImGui)
└── assets/           # Sprites, atlases, maps (JSON/PNG)
```

## 🧪 Running & Testing
### ▶️ Build & Run
```bash
cargo build
cargo run -p toki-editor
```
### ☑️  Run Tests
```bash
cargo test --workspace
```

### 📊 Code Coverage
```bash
cargo install cargo-llvm-cov
cargo llvm-cov -p toki-core --open

```
## License

This project is dual‑licensed under either:

- **GPL‑3.0‑or‑later** — see `LICENSE`; or
- **Toki Commercial License v1.0** — see `LICENSE-COMMERCIAL.md`.

Choose the option that fits your needs. To use this software in a proprietary product without GPL copyleft obligations, contact me to purchase a commercial license.
