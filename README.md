# Rookery

> **Living Worlds & Ambient Simulations for Your Desktop**  
> *Open-Source (MIT) • Cross-Platform (macOS & Windows) • AI-Native • Ultra-Lightweight*

**Rookery** is an open-source runtime engine and desktop host for interactive, procedural animations and slow-burn ambient simulations (**Rookery Apps**) that live quietly behind your desktop icons.

- **[Project Vision & Roadmap (`docs/VISION.md`)](docs/VISION.md)**
- **[Host Architecture & Developer Guide (`docs/architecture/host.md`)](docs/architecture/host.md)**
- **[Living ASCII Rookery Architecture (`docs/architecture/rookery-app.md`)](docs/architecture/rookery-app.md)**
- **[Performance Benchmarking & Packaging (`docs/architecture/bench-and-packaging.md`)](docs/architecture/bench-and-packaging.md)**
- **[Catalog, Open App SDK & Scheduler (`docs/architecture/catalog-and-sdk.md`)](docs/architecture/catalog-and-sdk.md)**

---

## Flagship Apps Catalog

| App | Archetype | Description |
| :--- | :--- | :--- |
| **🌲 The Living ASCII Rookery** | Botanical Slow-Burn | Procedural tree growth, foliage blooms, autonomous bird flock, birdhouses built over time, wooden plant signs, and horizontal forest panning. |
| **🎏 Tranquil Koi Pond** | Aquatic Zen Ecosystem | Articulated serpentine koi boids, blooming water lilies, floating food pellets, and procedural water ripple caustics. |
| **🌌 Cosmic Nebula** | Generative Physics & Shaders | Deep space interstellar dust, gravitational vortex attractors/repulsors, turbulence vector fields, and Hubble/JWST palette cycles. |
| **🛰️ Orbital Haven** | Slow-Burn Sci-Fi Space Station | Orbital planetary horizon with day/night terminator, modular station assembly, autonomous transport shuttles, and offline science research. |

---

## Features

### 🎛️ Open App SDK & Day-Night Scheduler
- **`rookery.toml` Manifest Specification**: Standard declarative configuration defining app metadata, dynamic UI controls (sliders, toggles, select dropdowns, action buttons), and resource budgets.
- **Playlist & Day/Night Scheduler**: Automatically rotate through apps at a fixed interval (e.g. every 15–60 mins) or transition smoothly between daytime nature worlds and night celestial skies.
- **Zero-Tear Hot-Switching**: Instantly switch apps across all active displays without display tearing or window recreation.
- **Global Popover Toggle Hotkey**: Toggle the menu popover using a global keyboard shortcut (`CommandOrControl+Alt+Shift+P` by default) without clicking the menu bar icon.

### 🌳 Living World Enhancements
- **Multi-Column Bark Reinforcement**: Sturdy, multi-column textured trunks for mature trees.
- **Birdhouse Construction**: Flocks build cozy wooden birdhouses on sturdy branches as trees grow.
- **Wooden Plant Signs**: Mark sapling planting locations with commemorative wooden signage.
- **Horizontal Forest Panning**: Pan left and right across the forest horizon directly from the popover.

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

