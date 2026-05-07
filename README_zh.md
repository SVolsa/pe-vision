# PE Vision

> 一个长得还不错的 PE 文件查看器 — Rust + egui

[English](README.md)

---

把 `.exe` / `.dll` / `.sys`（只要是 PE 文件都行）丢进去，看看里面长啥样。
头部、节区、导入表、导出表 — 深色主题配上视觉特效，看得清楚又好看。

## 能干啥

- **解析 PE 文件** — DOS 头 → NT 头 → 节表 → 导入表 → 导出表。纯手写 Rust，零依赖。
- **Hex 预览** — 点啥看啥。窗口式渲染，不会吃掉你的内存。
- **结构图** — 双行可视化 PE 布局，悬停看详情。
- **好看** — 深色主题、浮动粒子、平滑发光特效，都来了。

## 构建 & 运行

```bash
cargo build --release
cargo run --release
```

需要 Rust edition 2024。Windows + GNU 工具链需要 MinGW 库。

## 怎么用

1. 打开软件
2. **Open File**（或者直接把 PE 文件拖进去）
3. 左边树形结构点一点 → 右边显示详情和 Hex
4. 底下结构图悬停一下，全局一览

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
- **Claude** (Anthropic) — 码字搭子

## 许可证

MIT — 随便玩。