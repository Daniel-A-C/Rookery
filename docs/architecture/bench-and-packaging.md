# Performance Benchmarking, Sharing & Packaging (Milestone 3)

This document describes the automated performance harness, sharing tools, and CI/CD distribution pipelines implemented in Milestone 3 ([`docs/VISION.md`](../VISION.md) §5, §6, §7).

---

## 1. Overview & Architecture

```mermaid
flowchart TB
    subgraph CI["Automated CI/CD Pipeline"]
        Bench["Headless Performance Harness\n(rookery-bench)"]
        Budgets["Resource Budget Verification\n(RSS < 120MB, CPU tick < 2.0ms)"]
        VisualCheck["Visual Regression Assertions\n(Luma bounds + Frame Delta)"]
        ReleaseFlow["GitHub Actions Release\n(macOS .dmg + Windows .msi/.exe)"]
    end

    subgraph Sharing["Spreadability & Sharing Engine"]
        SeedURI["Seed Sharing\n(rookery://tree/seed-<hex>)"]
        StateURI["State Sharing\n(rookery://tree/state-<base64>)"]
        Snapshot["Clean PNG Snapshot Exporter\n(Offscreen Readback)"]
        GIFLoop["5-Second Animated GIF Loop\n(NeuQuant Color Quantization)"]
    end

    subgraph PopoverUI["Popover Controls (ui/popover)"]
        Buttons["Snapshot · Record GIF\nCopy Seed · Copy State · Import"]
    end

    Bench --> Budgets
    Bench --> VisualCheck
    Buttons --> Snapshot
    Buttons --> GIFLoop
    Buttons --> SeedURI
    Buttons --> StateURI
```

---

## 2. Headless Performance & Visual Regression Harness (`rookery-bench`)

The `rookery-bench` tool runs any Rookery scene deterministically offscreen in a headless buffer without opening OS desktop windows.

### 2.1 Telemetry Collection & Budget Assertions

During benchmark execution, `rookery-bench` records:
- **CPU Tick Time**: High-resolution measurement of `Scene::update(...)` per frame (mean, p95, maximum).
- **GPU Submission Time**: Command buffer encoding and queue submission duration.
- **Peak RSS (Memory)**: Native resident set size queried via `libc::getrusage` on macOS/Unix and `GetProcessMemoryInfo` on Windows.
- **Achieved Headless Simulation FPS**: Raw simulation throughput without display synchronization bottlenecks.

| Metric | Budget Target ([`VISION.md`](../VISION.md) §5.1) | Achieved Result (1,000 Ticks) | Status |
| :--- | :--- | :--- | :--- |
| **Peak Memory (RSS)** | `< 120.0 MB` | **35.45 MB** | Pass |
| **Mean CPU Tick Time** | `< 2.00 ms` | **0.06 ms** | Pass |
| **p95 CPU Tick Time** | `< 4.00 ms` | **0.07 ms** | Pass |
| **Simulation Throughput** | `> 60 FPS` | **~1,850 FPS** | Pass |

### 2.2 Visual Regression Protection

To catch visual regressions autonomously in headless CI environments:
- **Luma Bounds**: Ensures mean frame luminance is between $2.0$ (not pure black/blank) and $240.0$ (not blown out).
- **Procedural Evolution Check**: Calculates the per-pixel difference between successive snapshots (e.g. tick 0, 100, 500, 1000). A stalled scene ($< 0.01$ delta) immediately fails CI.
- **Artifact Export**: Snapshots can be saved as PNGs (`--snapshot-dir <path>`) alongside a comprehensive JSON telemetry summary (`--output-json <path>`).

### 2.3 CLI Usage

```bash
# Run 1,000 ticks and assert budgets (returns non-zero on failure)
cargo run -p rookery-bench -- --ticks 1000 --assert-budgets

# Export visual regression snapshots and JSON report
cargo run -p rookery-bench -- \
  --app living-rookery \
  --ticks 1000 \
  --snapshots 0,100,500,1000 \
  --snapshot-dir snapshots/ \
  --output-json telemetry.json \
  --assert-budgets
```

---

## 3. Viral Sharing & Export Tools

Milestone 3 equips Rookery with instant sharing tools right from the system tray / menu bar popover:

### 3.1 Shareable Seeds & World State URIs
- **World Seed URI (`rookery://tree/seed-<hex>`)**:
  Compact representation of the 64-bit world seed (e.g. `rookery://tree/seed-000000000000002a`). Allows friends to sprout the exact same tree lineage.
- **Full State Snapshot URI (`rookery://tree/state-<base64>`)**:
  Serializes the complete simulation state (all branches, growth progress, foliage buds, autonomous flock state, and age) into a URL-safe base64 string.
- **Universal Import**:
  Users can paste either a seed URI or a state URI into the popover's import box and click **Import** to immediately transform their desktop in real time.

### 3.2 Icon-Free Desktop Snapshots & GIF Loop Recorder
- **PNG Snapshot**:
  Renders an offscreen $1280 \times 720$ frame of the live world state using `rookery_render::capture::write_png`. Saves directly to `~/Pictures/Rookery/snapshot_YYYYMMDD_HHMMSS.png` with zero desktop icons or OS clutter.
- **5-Second GIF Loop**:
  Simulates and captures 60 frames (~5 seconds at 12.5 FPS) offscreen, encoding them via `rookery_render::capture::write_gif` with palette quantization. Saves directly to `~/Pictures/Rookery/loop_YYYYMMDD_HHMMSS.gif`.

---

## 4. One-Click Distribution & Packaging

- **GitHub Actions CI/CD (`.github/workflows/release.yml`)**:
  Automated multi-platform release workflow building native installers:
  - **macOS**: Universal Apple Silicon / Intel DMG (`.dmg`) with embedded `Info.plist` accessory configuration.
  - **Windows**: Modern MSI / NSIS installer package (`.msi` / `.exe`).
- **Continuous Performance Assurance (`.github/workflows/ci.yml`)**:
  Every PR and commit runs `rookery-bench --assert-budgets` to guarantee that no code change introduces performance or visual regressions.

