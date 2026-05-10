# PE Vision

> A minimal PE file viewer — Rust + egui

[中文](README_zh.md)

---

Drop any `.exe` / `.dll` / `.sys` (or any other PE file) into it, and see what's inside.
Added some rough visual effects for headers, sections, import table, export table, etc.

## What It Does

- **Parse PE Files** — From DOS header → NT header → Section Table → Import Table → Export Table. Pure hand-written Rust, zero dependencies.
- **Hex Preview** — Windowed rendering that won't eat up your memory.
- **Structural Diagram** — Dual-row visualization of PE layout, hover for details.


## Build & Run

```bash
cargo build --release
cargo run --release
```

Requires Rust edition 2024. Windows + GNU toolchain requires MinGW libraries.

## How to Use

1. Launch the software
2. **Open File** (or just drag a PE file into the window)
3. Hover over the structural diagram below for an overview

## Project Structure

```
src/
├── main.rs      — Entry point, dark theme
├── app.rs       — UI panels, tree view, async loading
├── pe.rs        — PE parser (pure Rust, no external dependencies)
├── hex.rs       — Hex viewer (smart windowed rendering)
└── visuals.rs   — Particles, glow effects, structural diagrams, loading animations
```

## Who Made This

- **Volsa** ([@SVolsa](https://github.com/SVolsa)) — Project & Code
- **Claude** (Anthropic) — Cleaning up my awful code

## License

MIT
