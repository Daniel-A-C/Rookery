# Rookery

> **Living Worlds & Ambient Simulations for Your Desktop**  
> *Open-Source (MIT) • Cross-Platform (macOS & Windows) • AI-Native • Ultra-Lightweight*

**Rookery** is an open-source runtime engine and desktop host for interactive, procedural animations and slow-burn ambient simulations (**Rookery Apps**) that live quietly behind your desktop icons.

- **[Project Vision & Roadmap (`docs/VISION.md`)](docs/VISION.md)**
- **[Host Architecture & Developer Guide (`docs/architecture/host.md`)](docs/architecture/host.md)**

---

## Milestone 1 Features

- **Native Desktop Layer Attachment**:
  - **macOS**: Attaches below desktop icons (`kCGDesktopWindowLevelKey`), spans all Spaces/Mission Control cleanly, and runs as a Dock-less menu bar accessory.
  - **Windows**: Attaches to `Progman` / `WorkerW` behind `SHELLDLL_DefView`, supporting both classic Windows 10 / 11 (<=23H2) and Windows 11 24H2+ desktop hierarchies.
- **Zero-WebView Wallpaper Rendering (`wgpu`)**:
  - One shared low-power `wgpu::Device` and a single render thread driving per-monitor surfaces without any background WebView process overhead.
- **Smart Performance Governor**:
  - Automatically drops occluded displays to `0 FPS` (blocking on the render channel with `0.0%` CPU) when covered by maximized or fullscreen windows, and adapts frame pacing to AC, Battery, and Low-Power states.
- **Dual-Mode Interaction Architecture**:
  - **System Tray / Menu Bar Popover**: On-demand lightweight control popover (created on click, destroyed on focus loss) for inspecting per-display FPS/occlusion status, pausing, and tuning frame budgets.
  - **Direct Interaction Mode**: Toggle via global hotkey (`Cmd+Opt+Shift+R` on macOS / `Ctrl+Alt+Shift+R` on Windows) to promote the desktop simulation above icons and interact with the cursor; press `Esc` or click any application window to return to click-through passive mode.

---

## Quick Start

```bash
# 1. Install frontend dependencies & build the tray popover UI
npm install
npm run build:ui

# 2. Run unit tests across the workspace
cargo test --workspace

# 3. Launch Rookery in development mode
npm run dev

# 4. Build optimized release binary
cargo build -p rookery-host --release
```

## License

Licensed under the [MIT License](LICENSE).

