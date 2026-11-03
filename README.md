# Rookery

> **Living Worlds & Ambient Simulations for Your Desktop**  
> *Open-Source (MIT) • Cross-Platform (macOS & Windows) • AI-Native • Ultra-Lightweight*

**Rookery** is an open-source runtime engine and desktop host for interactive, procedural animations and slow-burn ambient simulations (**Rookery Apps**) that live quietly behind your desktop icons.

- **[Project Vision & Roadmap (`docs/VISION.md`)](docs/VISION.md)**
- **[Host Architecture & Developer Guide (`docs/architecture/host.md`)](docs/architecture/host.md)**
- **[Living ASCII Rookery Architecture (`docs/architecture/rookery-app.md`)](docs/architecture/rookery-app.md)**
- **[Performance Benchmarking & Packaging (`docs/architecture/bench-and-packaging.md`)](docs/architecture/bench-and-packaging.md)**

---

## Features

### 🌳 The Living ASCII Rookery (Flagship App)
- **Overlapping SDF Glyph Renderer**:
  Crisp, sub-pixel text rendering with smooth wind sway and dynamic scaling powered by a GPU-accelerated Signed Distance Field (SDF) glyph atlas.
- **Procedural Growth Algorithm**:
  Trunk and branching systems expand organically over days and weeks, sprouting lush foliage buds and blooming flowers.
- **Autonomous Wildlife (The Flock)**:
  Procedural birds glide across the desktop, circle the canopy, perch on branches, feed on scattered snacks, and sing floating musical notes.
- **Offline Time-Delta Catch-Up**:
  Deterministic catch-up engine that advances world growth seamlessly when the computer wakes from sleep.

### ⚡ Performance & Resource Governance
- **Strict Budgets**: `< 120 MB` peak RAM, `< 1.5%` background CPU usage, and automated headless CI benchmarks via `rookery-bench`.
- **Smart Occlusion Detection**: Pauses rendering to `0 FPS` when covered by maximized or fullscreen windows.
- **Power-Aware Pacing**: Dynamically throttles tick rates when running on battery or low-power modes.

### 📸 Sharing, Viral Export & Distribution
- **One-Click Snapshots & GIF Loops**: Export clean, icon-free PNG screenshots and 5-second animated GIF loops directly to `~/Pictures/Rookery`.
- **Shareable Seeds & State URIs**: Copy `rookery://tree/seed-<hex>` or full snapshot `rookery://tree/state-<base64>` strings to share worlds with friends or restore past ecosystems.
- **Cross-Platform One-Click Installers**: Automated GitHub Actions releases building Universal macOS `.dmg` and Windows `.msi` / `.exe` installers.

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

