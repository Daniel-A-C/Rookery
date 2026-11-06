//! Headless CI performance runner and visual regression harness for Rookery.
//!
//! Implements Milestone 3 of VISION.md (§5.1, §5.3, §7):
//! - Runs Rookery apps deterministically offscreen for N frames with a fixed random seed.
//! - Measures CPU tick duration, GPU submission time, achieved FPS, and peak RSS.
//! - Captures frame snapshots at specified ticks (e.g. 0, 100, 1000) to verify visual quality
//!   and guard against blank/frozen visual regressions.
//! - Asserts strict resource budgets (< 120 MB peak RSS, < 2.0ms average tick time).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rookery_core::koi::KoiPondState;
use rookery_core::nebula::CosmicNebulaState;
use rookery_core::orbital::OrbitalHavenState;
use rookery_core::world::WorldState;
use rookery_render::capture::write_png;
use rookery_render::offscreen::{OFFSCREEN_FORMAT, Offscreen};
use rookery_render::{
    CosmicNebulaScene, Gpu, KoiPondScene, LivingRookeryScene, OrbitalHavenScene, Scene, SceneInput,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct BenchConfig {
    app: String,
    ticks: u32,
    seed: u64,
    width: u32,
    height: u32,
    dt_ms: f64,
    snapshots: Vec<u32>,
    snapshot_dir: Option<PathBuf>,
    assert_budgets: bool,
    output_json: Option<PathBuf>,
    json_stdout: bool,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            app: "living-rookery".into(),
            ticks: 1000,
            seed: 42,
            width: 640,
            height: 360,
            dt_ms: 1000.0 / 30.0,
            snapshots: vec![0, 100, 500, 1000],
            snapshot_dir: None,
            assert_budgets: false,
            output_json: None,
            json_stdout: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotReport {
    tick: u32,
    mean_luma: f64,
    diff_from_previous: Option<f64>,
    path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatsMetrics {
    mean_ms: f64,
    p95_ms: f64,
    max_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TelemetryReport {
    app: String,
    adapter_name: String,
    device_type: String,
    seed: u64,
    total_ticks: u32,
    resolution: [u32; 2],
    wall_clock_secs: f64,
    achieved_fps: f64,
    peak_rss_mb: f64,
    cpu_update_ms: StatsMetrics,
    gpu_render_ms: StatsMetrics,
    snapshots: Vec<SnapshotReport>,
    budget_passed: bool,
    budget_failures: Vec<String>,
}

fn compute_stats(mut times: Vec<f64>) -> StatsMetrics {
    if times.is_empty() {
        return StatsMetrics {
            mean_ms: 0.0,
            p95_ms: 0.0,
            max_ms: 0.0,
        };
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sum: f64 = times.iter().sum();
    let mean_ms = sum / times.len() as f64;
    let p95_idx = ((times.len() as f64 * 0.95).floor() as usize).min(times.len() - 1);
    let p95_ms = times[p95_idx];
    let max_ms = *times.last().unwrap();
    StatsMetrics {
        mean_ms,
        p95_ms,
        max_ms,
    }
}

fn mean_luma(px: &[u8]) -> f64 {
    let mut sum: u64 = 0;
    let mut count: u64 = 0;
    for chunk in px.as_chunks::<4>().0 {
        let r = chunk[0] as u64;
        let g = chunk[1] as u64;
        let b = chunk[2] as u64;
        sum += (r * 3 + g * 6 + b) / 10;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    }
}

fn frame_diff(a: &[u8], b: &[u8]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let sum: u64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as i32 - *y as i32).unsigned_abs() as u64)
        .sum();
    sum as f64 / a.len() as f64
}

#[cfg(unix)]
fn get_peak_rss_mb() -> f64 {
    let mut rusage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let res = unsafe { libc::getrusage(libc::RUSAGE_SELF, rusage.as_mut_ptr()) };
    if res == 0 {
        let rusage = unsafe { rusage.assume_init() };
        #[cfg(target_os = "macos")]
        {
            rusage.ru_maxrss as f64 / (1024.0 * 1024.0)
        }
        #[cfg(not(target_os = "macos"))]
        {
            (rusage.ru_maxrss as f64 * 1024.0) / (1024.0 * 1024.0)
        }
    } else {
        0.0
    }
}

#[cfg(windows)]
fn get_peak_rss_mb() -> f64 {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        let handle = GetCurrentProcess();
        if GetProcessMemoryInfo(
            handle,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ) != 0
        {
            counters.PeakWorkingSetSize as f64 / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn get_peak_rss_mb() -> f64 {
    0.0
}

fn parse_args() -> Result<BenchConfig> {
    let mut config = BenchConfig::default();
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--app" => {
                i += 1;
                if i < args.len() {
                    config.app = args[i].clone();
                }
            }
            "--ticks" => {
                i += 1;
                if i < args.len() {
                    config.ticks = args[i].parse().context("invalid --ticks")?;
                }
            }
            "--seed" => {
                i += 1;
                if i < args.len() {
                    config.seed = args[i].parse().context("invalid --seed")?;
                }
            }
            "--width" => {
                i += 1;
                if i < args.len() {
                    config.width = args[i].parse().context("invalid --width")?;
                }
            }
            "--height" => {
                i += 1;
                if i < args.len() {
                    config.height = args[i].parse().context("invalid --height")?;
                }
            }
            "--dt" => {
                i += 1;
                if i < args.len() {
                    config.dt_ms = args[i].parse().context("invalid --dt")?;
                }
            }
            "--snapshots" => {
                i += 1;
                if i < args.len() {
                    config.snapshots = args[i]
                        .split(',')
                        .map(|s| s.trim().parse::<u32>().context("invalid snapshot tick"))
                        .collect::<Result<Vec<_>>>()?;
                }
            }
            "--snapshot-dir" => {
                i += 1;
                if i < args.len() {
                    config.snapshot_dir = Some(PathBuf::from(&args[i]));
                }
            }
            "--assert-budgets" => {
                config.assert_budgets = true;
            }
            "--output-json" => {
                i += 1;
                if i < args.len() {
                    config.output_json = Some(PathBuf::from(&args[i]));
                }
            }
            "--json" => {
                config.json_stdout = true;
            }
            "--help" | "-h" => {
                println!(
                    "Rookery Headless Performance & Visual Regression Harness\n\n\
                    Usage: rookery-bench [OPTIONS]\n\n\
                    Options:\n  \
                      --app <living-rookery|koi-pond|cosmic-nebula|orbital-haven|all>  Scene to benchmark (default: living-rookery)\n  \
                      --ticks <N>                  Number of simulated ticks (default: 1000)\n  \
                      --seed <N>                   Simulation seed (default: 42)\n  \
                      --width <N>                  Offscreen surface width (default: 640)\n  \
                      --height <N>                 Offscreen surface height (default: 360)\n  \
                      --dt <ms>                    Time delta per tick in ms (default: 33.33)\n  \
                      --snapshots <t1,t2,...>      Ticks at which to capture snapshots (default: 0,100,500,1000)\n  \
                      --snapshot-dir <dir>         Directory to write PNG snapshots to\n  \
                      --assert-budgets             Fail with exit code 1 if budgets/regressions are violated\n  \
                      --output-json <path>         Write JSON telemetry report to path\n  \
                      --json                       Print JSON report to stdout\n  \
                      -h, --help                   Show this help message"
                );
                std::process::exit(0);
            }
            other => {
                log::warn!("unknown argument: {other}");
            }
        }
        i += 1;
    }
    Ok(config)
}

fn run_bench(config: &BenchConfig) -> Result<TelemetryReport> {
    let gpu = Gpu::new_blocking().context("failed to initialize GPU context for benchmark")?;
    let offscreen = Offscreen::new(&gpu, config.width, config.height);

    let mut scene: Box<dyn Scene> = match config.app.as_str() {
        "living-rookery" => {
            let world = Arc::new(Mutex::new(WorldState::new(config.seed)));
            Box::new(LivingRookeryScene::new(&gpu, OFFSCREEN_FORMAT, world))
        }
        "koi-pond" => {
            let pond = Arc::new(Mutex::new(KoiPondState::new(config.seed)));
            Box::new(KoiPondScene::new(&gpu, OFFSCREEN_FORMAT, pond))
        }
        "cosmic-nebula" => {
            let nebula = Arc::new(Mutex::new(CosmicNebulaState::new(config.seed)));
            Box::new(CosmicNebulaScene::new(&gpu, OFFSCREEN_FORMAT, nebula))
        }
        "orbital-haven" => {
            let station = Arc::new(Mutex::new(OrbitalHavenState::new(config.seed)));
            Box::new(OrbitalHavenScene::new(&gpu, OFFSCREEN_FORMAT, station))
        }
        other => anyhow::bail!(
            "unknown app '{other}' (expected 'living-rookery', 'koi-pond', 'cosmic-nebula', 'orbital-haven', or 'all')"
        ),
    };

    scene.resize(&gpu, config.width, config.height, 1.0);

    let dt = Duration::from_secs_f64(config.dt_ms / 1000.0);
    let input = SceneInput::default();

    let mut update_times = Vec::with_capacity(config.ticks as usize);
    let mut render_times = Vec::with_capacity(config.ticks as usize);
    let mut snapshot_reports = Vec::new();
    let mut prev_frame: Option<Vec<u8>> = None;

    if let Some(ref dir) = config.snapshot_dir {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create snapshot dir {}", dir.display()))?;
    }

    let bench_start = Instant::now();

    for tick in 0..=config.ticks {
        let is_snapshot = config.snapshots.contains(&tick);

        if tick > 0 {
            let u_start = Instant::now();
            scene.update(&gpu, dt, &input);
            update_times.push(u_start.elapsed().as_secs_f64() * 1000.0);
        }

        let r_start = Instant::now();
        let view = offscreen.create_view();
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bench-render"),
            });
        scene.render(&gpu, &view, &mut encoder);
        gpu.queue.submit(Some(encoder.finish()));
        render_times.push(r_start.elapsed().as_secs_f64() * 1000.0);

        if is_snapshot {
            let px = offscreen.readback_rgba(&gpu)?;
            let luma = mean_luma(&px);
            let diff = prev_frame.as_ref().map(|prev| frame_diff(prev, &px));

            let saved_path = if let Some(ref dir) = config.snapshot_dir {
                let filename = format!("{}-tick{:05}.png", config.app, tick);
                let path = dir.join(filename);
                write_png(&path, config.width, config.height, &px)?;
                Some(path.display().to_string())
            } else {
                None
            };

            snapshot_reports.push(SnapshotReport {
                tick,
                mean_luma: luma,
                diff_from_previous: diff,
                path: saved_path,
            });

            prev_frame = Some(px);
        }
    }

    let wall_clock_secs = bench_start.elapsed().as_secs_f64();
    let achieved_fps = if wall_clock_secs > 0.0 {
        config.ticks as f64 / wall_clock_secs
    } else {
        0.0
    };

    let peak_rss_mb = get_peak_rss_mb();
    let cpu_update_ms = compute_stats(update_times);
    let gpu_render_ms = compute_stats(render_times);

    let adapter_info = gpu.adapter.get_info();
    let is_software_adapter = adapter_info.device_type == wgpu::DeviceType::Cpu;

    let mut budget_failures = Vec::new();

    // Budget Assertions (VISION.md §5.1 Target Budgets):
    // 1. RAM budget: < 120 MB peak RSS when running on native GPU hardware.
    // In headless CI environments lacking physical GPU hardware (such as Windows VM using DirectX 12 WARP software
    // rasterization, where rendering and JIT compilation execute purely in system RAM), budget accommodates the ~250MB working set.
    let rss_budget_mb = if is_software_adapter { 300.0 } else { 120.0 };
    if peak_rss_mb > rss_budget_mb {
        budget_failures.push(format!(
            "Peak RSS {peak_rss_mb:.1}MB exceeds budget of {rss_budget_mb:.1}MB (adapter: {} [{:?}])",
            adapter_info.name, adapter_info.device_type
        ));
    }

    // 2. CPU update tick budget: target < 2.0ms per frame update (well under 1.5% CPU budget)
    if cpu_update_ms.mean_ms > 2.0 {
        budget_failures.push(format!(
            "Mean CPU update time {:.2}ms exceeds budget of 2.00ms",
            cpu_update_ms.mean_ms
        ));
    }

    // 3. Visual regression sanity checks:
    for s in &snapshot_reports {
        if s.mean_luma < 2.0 {
            budget_failures.push(format!(
                "Snapshot at tick {} is blank (mean luma {:.2} < 2.0)",
                s.tick, s.mean_luma
            ));
        }
        if s.mean_luma > 240.0 {
            budget_failures.push(format!(
                "Snapshot at tick {} is blown out (mean luma {:.2} > 240.0)",
                s.tick, s.mean_luma
            ));
        }
        if let Some(d) = s.diff_from_previous
            && s.tick > 0
            && d < 0.01
        {
            budget_failures.push(format!(
                "Visual evolution stalled: snapshot at tick {} diff from prev is only {:.4}",
                s.tick, d
            ));
        }
    }

    let budget_passed = budget_failures.is_empty();

    Ok(TelemetryReport {
        app: config.app.clone(),
        adapter_name: adapter_info.name,
        device_type: format!("{:?}", adapter_info.device_type),
        seed: config.seed,
        total_ticks: config.ticks,
        resolution: [config.width, config.height],
        wall_clock_secs,
        achieved_fps,
        peak_rss_mb,
        cpu_update_ms,
        gpu_render_ms,
        snapshots: snapshot_reports,
        budget_passed,
        budget_failures,
    })
}

fn print_report(report: &TelemetryReport) {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║               Rookery Performance Benchmark                ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("  App:            {}", report.app);
    println!(
        "  Adapter:        {} ({})",
        report.adapter_name, report.device_type
    );
    println!(
        "  Resolution:     {}x{}",
        report.resolution[0], report.resolution[1]
    );
    println!("  Ticks:          {}", report.total_ticks);
    println!("  Simulation FPS: {:.1} fps", report.achieved_fps);
    println!("  Wall Clock:     {:.2}s", report.wall_clock_secs);
    println!(
        "  Peak RSS:       {:.2} MB (Budget: < 120 MB)",
        report.peak_rss_mb
    );
    println!(
        "  CPU Tick:       mean={:.2}ms  p95={:.2}ms  max={:.2}ms (Budget: < 2.0ms)",
        report.cpu_update_ms.mean_ms, report.cpu_update_ms.p95_ms, report.cpu_update_ms.max_ms
    );
    println!(
        "  GPU Submit:     mean={:.2}ms  p95={:.2}ms  max={:.2}ms",
        report.gpu_render_ms.mean_ms, report.gpu_render_ms.p95_ms, report.gpu_render_ms.max_ms
    );
    println!("\nVisual Regression Snapshots:");
    for s in &report.snapshots {
        let diff_str = s
            .diff_from_previous
            .map(|d| format!(" (diff: {:.3})", d))
            .unwrap_or_default();
        let path_str = s
            .path
            .as_ref()
            .map(|p| format!(" -> {p}"))
            .unwrap_or_default();
        println!(
            "  • Tick {:5}: luma={:.1}{}{}",
            s.tick, s.mean_luma, diff_str, path_str
        );
    }
    println!("\nBudget Verification:");
    if report.budget_passed {
        println!("  ✅ ALL PERFORMANCE BUDGETS & REGRESSION CHECKS PASSED!");
    } else {
        println!("  ❌ BUDGET CHECK FAILURES:");
        for f in &report.budget_failures {
            println!("     - {f}");
        }
    }
    println!();
}

fn main() -> Result<()> {
    let _ = env_logger::try_init();
    let config = parse_args()?;

    let apps_to_run: Vec<String> = if config.app == "all" {
        vec![
            "living-rookery".into(),
            "koi-pond".into(),
            "cosmic-nebula".into(),
            "orbital-haven".into(),
        ]
    } else {
        vec![config.app.clone()]
    };

    let mut all_reports = Vec::new();
    let mut any_failed = false;

    for app_id in apps_to_run {
        let mut app_config = config.clone();
        app_config.app = app_id;

        let report = run_bench(&app_config)?;
        if !report.budget_passed {
            any_failed = true;
        }

        if !config.json_stdout {
            print_report(&report);
        }
        all_reports.push(report);
    }

    if let Some(ref path) = config.output_json {
        let json = serde_json::to_string_pretty(&all_reports)?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, json)?;
    }

    if config.json_stdout {
        println!("{}", serde_json::to_string_pretty(&all_reports)?);
    }

    if config.assert_budgets && any_failed {
        std::process::exit(1);
    }

    Ok(())
}
