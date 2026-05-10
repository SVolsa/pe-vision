# PE Vision

> 一个精简 PE 文件查看器 — Rust + egui

[English](README.md)

---

把 `.exe` / `.dll` / `.sys`（只要是 PE 文件都行）丢进去，看看里面长啥样
头部、节区、导入表、导出表 什么的写了一点粗糙特效

## 能干啥

- **解析 PE 文件** — DOS 头 → NT 头 → 节表 → 导入表 → 导出表。纯手写 Rust，零依赖
- **Hex 预览** — 窗口式渲染，不会吃掉你的内存
- **结构图** — 双行可视化 PE 布局，悬停看详情


## 构建 & 运行

```bash
cargo build --release
cargo run --release
```

需要 Rust edition 2024。Windows + GNU 工具链需要 MinGW 库。

## 怎么用

1. 打开软件
2. **Open File**（或者直接把 PE 文件拖进去）
3. 底下结构图悬停一下，全局一览

## 项目结构

```
src/
├── main.rs      — 入口、深色主题
├── app.rs       — UI 面板、树、异步加载
├── pe.rs        — PE 解析器（纯 Rust，无外部依赖）
├── hex.rs       — Hex 查看器（智能窗口渲染）
└── visuals.rs   — 粒子、发光、结构图、加载动画
```

## 谁写的

- **Volsa** ([@SVolsa](https://github.com/SVolsa)) — 项目 & 代码
- **Claude** (Anthropic) — 修我写的狗屎

## 许可证

MIT
