# Rookery Host Architecture (Milestone 1)

This document describes the architecture, runtime behavior, and platform specifics of the **Rookery Core Desktop Host** implemented in Milestone 1 ([`docs/VISION.md`](../VISION.md) §7).

---

## 1. Crate & Workspace Map

```
Rookery/
├── crates/
│   ├── rookery-core/       # Pure, deterministic, OS/GPU-free logic (governor, interaction, occlusion math, settings)
│   ├── rookery-platform/   # Thin OS layer: DesktopLayer, OcclusionProbe, PowerProbe (macOS & Windows)
│   └── rookery-render/     # Shared wgpu Device + single render thread driving N display surfaces + DemoScene
├── host/                   # Tauri v2 desktop host (tray, on-demand popover webview, hotkeys, monitor reconciler)
└── ui/popover/             # Vite + vanilla TypeScript tray popover UI (~12 KB uncompressed)
```

### Design Principle: Testable Core, Thin Edges
All decision-making lives in `rookery-core`, which has zero OS or GPU dependencies and runs its 30 unit tests in ~20ms on any CI runner:
- [`governor.rs`](../../crates/rookery-core/src/governor.rs): Pure function from `GovernorInputs` to per-display `FrameBudget` (`Paused(PauseReason)` or `Fps(u32)`).
- [`interaction.rs`](../../crates/rookery-core/src/interaction.rs): `Passive ⇄ Interactive` state machine returning ordered `Effect` values (`Promote`, `Demote`, `ShowIndicator(bool)`, `ListenForEscape(bool)`).
- [`occlusion.rs`](../../crates/rookery-core/src/occlusion.rs): Exact rectangle-union coverage calculation via a coordinate-compressed 1D sweep.
- [`settings.rs`](../../crates/rookery-core/src/settings.rs): Versioned JSON settings (`settings.json`) with atomic write-temp-fsync-rename saves, corrupt-file backup recovery, and `last_run_utc` / `last_shutdown_clean` heartbeat tracking for Milestone 2's offline time-delta engine.

---

## 2. Threading & Memory Model

| Thread | Responsibility | Idle CPU when Occluded |
| :--- | :--- | :--- |
| **Main thread (Tauri event loop)** | Owns native `NSWindow` / `HWND` handles, tray icon, global shortcut dispatch, and monitor reconciliation. | `~0.0%` |
| **`rookery-render` thread** | Owns `wgpu::Surface` instances and `Scene` instances across all displays. Sleeps until the earliest display deadline; when all displays are `Paused`, blocks indefinitely on `mpsc::Receiver::recv()`. | `0.0%` |
| **`rookery-governor` thread (1 Hz)** | Queries `OcclusionProbe`, `PowerProbe`, and `screens_asleep()` once per second, updates per-display `FrameBudget`s, and reconciles monitor hotplug every 2s. | `< 0.1%` |

### Why WebView-less Wallpaper Windows?
Wallpaper surfaces are created with `tauri::window::WindowBuilder` (enabled via Tauri's `unstable` feature flag) rather than `WebviewWindowBuilder`. Each monitor gets a bare native window rendered directly by `wgpu` (choosing `PowerPreference::LowPower` so dual-GPU laptops stay on integrated graphics).

Meanwhile, the tray popover (`WebviewWindow` labeled `"popover"`) is **created on demand when the tray icon is clicked and destroyed immediately on focus loss**. During normal passive operation, **zero WebView processes exist**, keeping RAM well inside the `< 120 MB` RSS budget.

---

## 3. Platform Notes

### macOS (`crates/rookery-platform/src/macos.rs`)
- **Desktop Layer Attachment**: Configures each wallpaper `NSWindow` with `setLevel(CGWindowLevelForKey(kCGDesktopWindowLevelKey))` (below `kCGDesktopIconWindowLevel`, so desktop icons sit above the wallpaper and remain clickable), `NSWindowCollectionBehavior::CanJoinAllSpaces | Stationary | IgnoresCycle | FullScreenNone`, and `setIgnoresMouseEvents(true)`.
- **Direct Interaction Mode**: Promotes the window to `kCGDesktopIconWindowLevel + 1` (above desktop icons, below normal app windows), enables mouse events, activates the app, and listens for `Escape` or focus loss (`WindowEvent::Focused(false)`) to demote back to passive mode.
- **Occlusion Detection**: Combines `NSWindow::occlusionState()` (which immediately detects macOS native Fullscreen Spaces) with `CGWindowListCopyWindowInfo(OptionOnScreenOnly | ExcludeDesktopElements)` filtered to layer-0 windows with `alpha >= 0.85` not owned by Rookery's PID.

### Windows (`crates/rookery-platform/src/windows.rs`)
- **Desktop Layer Attachment (Win10, Win11 <=23H2, and Win11 24H2+)**:
  - Sends message `0x052C` (`WPARAM(0xD), LPARAM(0x1)`) to `Progman` to spawn the wallpaper `WorkerW`.
  - Detects **Windows 11 24H2+**, where `SHELLDLL_DefView` and `WorkerW` are both children of `Progman`, and inserts the child `HWND` immediately below `SHELLDLL_DefView` in Z-order.
  - Falls back to the classic **<=23H2** hierarchy where a top-level `WorkerW` sibling follows the `WorkerW` hosting `SHELLDLL_DefView`.
- **Occlusion Detection**: Combines `SHQueryUserNotificationState()` (`QUNS_RUNNING_D3D_FULL_SCREEN`, `QUNS_BUSY`, `QUNS_PRESENTATION_MODE`) with `EnumWindows` + `DwmGetWindowAttribute(DWMWA_CLOAKED)` (ignoring windows on other virtual desktops or suspended UWP shells) and `DWMWA_EXTENDED_FRAME_BOUNDS`.

---

## 4. Development & Verification Commands

```bash
# Install frontend dependencies and build the tray popover UI
npm install
npm run build:ui

# Format, lint, and run all unit tests
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Run the GPU offscreen smoke test (renders DemoScene at ticks 0, 100, 1000)
cargo test -p rookery-render -- --ignored

# Launch Rookery in development mode
npm run dev

# Build optimized release binary
cargo build -p rookery-host --release
```

