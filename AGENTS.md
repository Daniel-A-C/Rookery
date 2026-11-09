# Agent Guidelines & CI Verification Protocol

## Mandatory Pre-Completion CI Verification Gate
Before declaring any task complete or asking to commit/push to remote, you **MUST** run the local CI verification checks and ensure all formatting, clippy warnings, tests, benchmarks, and UI builds pass cleanly:

```bash
cargo fmt --all && npm run check:ci
```

### Breakdown of Verification Checks
If you prefer running individual checks or need to isolate an issue:
1. **Format Code**:
   ```bash
   cargo fmt --all -- --check
   ```
2. **Clippy (Strict - Zero Warnings)**:
   CI enforces `-D warnings`. All targets across the workspace must pass without warnings:
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```
3. **Workspace Tests**:
   ```bash
   cargo test --workspace
   ```
4. **Performance Budgets & Headless Benchmark**:
   ```bash
   cargo run -p rookery-bench -- --ticks 200 --assert-budgets
   ```
5. **Popover UI Build**:
   ```bash
   npm run build:ui
   ```

## Cross-Platform Awareness
- CI runs on both **`macos-latest`** and **`windows-latest`**.
- Never assume macOS-specific APIs or file paths. Always use `#[cfg(target_os = "macos")]` / `#[cfg(target_os = "windows")]` or platform abstractions in `rookery-platform`.
- When adding new crate dependencies, ensure licenses comply with [deny.toml](file:///Users/danielcarroll/Documents/programs/Rookery/deny.toml) (`cargo-deny check licenses sources`).

