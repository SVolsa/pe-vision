# PE Vision

> A fancy PE file snooper 🔍 — Rust + egui

[中文](README_zh.md)

---

Throw a `.exe` / `.dll` / `.sys` (or any PE file) at it and see what's inside.  
Headers, sections, imports, exports — all laid out with a dark theme and some eye candy.

![License](https://img.shields.io/badge/license-MIT-blue)

## What it does

- **Parses PE files** — DOS header → NT headers → sections → imports → exports. All hand-written Rust, zero deps.
- **Hex preview** — Click anything, see the bytes. Smart windowing so it won't eat your RAM.
- **Structure map** — Two-row visual of the whole PE layout. Hover for details.
- **Looks pretty** — Dark theme, floating particles, smooth hover glows. Because why not.

## Build & run

```bash
cargo build --release
cargo run --release
```

Needs Rust edition 2024. On Windows + GNU toolchain you'll want MinGW libs.

## Usage

1. Open the app
2. **Open File** (or drag & drop a PE file)
3. Click around the tree on the left → details + hex pop up on the right
4. Hover the structure map at the bottom for a bird's-eye view

## Project skeleton

```
src/
├── main.rs      — entry point, dark theme
├── app.rs       — UI panels, tree, async loading
├── pe.rs        — PE parser (pure Rust, no helpers)
├── hex.rs       — hex viewer with smart windowing
└── visuals.rs   — particles, glow, structure map, spinner
```

## Who

- **Volsa** ([@SVolsa](https://github.com/SVolsa)) — project & code
- **Claude** (Anthropic) — coding buddy

## License

MIT — do whatever.
