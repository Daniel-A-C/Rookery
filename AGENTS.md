# Agent Guidelines & CI Verification Protocol

## Mandatory Pre-Completion CI Verification Gate
Before declaring any task complete or asking to commit/push to remote, you **MUST** run the local CI verification checks **after your final edit** and ensure all formatting, clippy warnings, tests, benchmarks, and UI builds pass cleanly:

```bash
cargo fmt --all && npm run check:ci
```

### Clean-Commit & Push Discipline
Most past GitHub CI failures were caused by either (a) committing only a subset of modified files while dependent edits remained uncommitted in the working tree, or (b) making edits after running `check:ci` without re-running it.
- **Check `git status --short` before committing**: verify that every modified file required by the build is staged. Never run `check:ci` against a dirty working tree and then selectively commit only part of it.
- **Re-run after any edit**: if you touch any file after running `npm run check:ci`, re-run `cargo fmt --all && npm run check:ci` before committing.
- **Keep `.githooks/pre-push` active**: this repo ships a versioned `.githooks/pre-push` hook (installed via `npm run hooks:install`) that blocks pushes with uncommitted tracked changes and runs `npm run check:ci`. Do **not** bypass it with `--no-verify`.
- **Toolchain parity**: the Rust version is pinned in [rust-toolchain.toml](file:///Users/danielcarroll/Documents/programs/Rookery/rust-toolchain.toml) and matched in [.github/workflows/ci.yml](file:///Users/danielcarroll/Documents/programs/Rookery/.github/workflows/ci.yml) so local `rustfmt` and `clippy` match CI lint-for-lint. If you bump the version in one, bump both together.

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

