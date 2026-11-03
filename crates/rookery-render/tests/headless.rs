//! Headless smoke test: render the demo scene offscreen at several ticks and check that
//! frames are non-blank and evolve over time. Requires a GPU, so it is `#[ignore]` by
//! default. Run with: `cargo test -p rookery-render -- --ignored`.
//!
//! Set `ROOKERY_SNAPSHOT_DIR=/some/dir` to also dump raw RGBA8 frames (`.rgba`) + a PPM preview.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rookery_core::world::WorldState;
use rookery_render::demo::DemoScene;
use rookery_render::offscreen::{OFFSCREEN_FORMAT, Offscreen};
use rookery_render::{Gpu, LivingRookeryScene, Scene};

const W: u32 = 320;
const H: u32 = 180;

fn mean_luma(px: &[u8]) -> f64 {
    let sum: u64 = px
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| (p[0] as u64 * 3 + p[1] as u64 * 6 + p[2] as u64) / 10)
        .sum();
    sum as f64 / (px.len() / 4) as f64
}

fn diff(a: &[u8], b: &[u8]) -> f64 {
    let sum: u64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| (*x as i32 - *y as i32).unsigned_abs() as u64)
        .sum();
    sum as f64 / a.len() as f64
}

fn write_ppm(path: &std::path::Path, px: &[u8]) {
    let mut out = format!("P6\n{W} {H}\n255\n").into_bytes();
    for p in px.as_chunks::<4>().0 {
        out.extend_from_slice(&p[..3]);
    }
    std::fs::write(path, out).unwrap();
}

#[test]
#[ignore = "requires a GPU"]
fn demo_scene_renders_and_evolves() {
    let gpu = Gpu::new_blocking().expect("GPU");
    let mut scene = DemoScene::new(&gpu, OFFSCREEN_FORMAT, 7);
    scene.resize(&gpu, W, H, 1.0);
    let off = Offscreen::new(&gpu, W, H);
    let dt = Duration::from_millis(1000 / 24);

    let f0 = off.run(&gpu, &mut scene, 1, dt).unwrap();
    let f100 = off.run(&gpu, &mut scene, 99, dt).unwrap();
    let f1000 = off.run(&gpu, &mut scene, 900, dt).unwrap();

    for (name, f) in [("t0", &f0), ("t100", &f100), ("t1000", &f1000)] {
        let l = mean_luma(f);
        assert!(l > 2.0, "{name} looks blank (mean luma {l})");
        assert!(l < 200.0, "{name} looks blown out (mean luma {l})");
    }
    assert!(
        diff(&f0, &f100) > 0.05,
        "frame did not change between t0 and t100"
    );
    assert!(
        diff(&f100, &f1000) > 0.05,
        "frame did not change between t100 and t1000"
    );

    if let Ok(dir) = std::env::var("ROOKERY_SNAPSHOT_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, f) in [("t0", &f0), ("t100", &f100), ("t1000", &f1000)] {
            write_ppm(&dir.join(format!("demo-{name}.ppm")), f);
        }
    }
}

#[test]
#[ignore = "requires a GPU"]
fn living_rookery_scene_renders_and_evolves() {
    let gpu = Gpu::new_blocking().expect("GPU");
    let world = Arc::new(Mutex::new(WorldState::new(42)));
    let mut scene = LivingRookeryScene::new(&gpu, OFFSCREEN_FORMAT, world);
    scene.resize(&gpu, W, H, 1.0);
    let off = Offscreen::new(&gpu, W, H);
    let dt = Duration::from_millis(1000 / 24);

    let f0 = off.run(&gpu, &mut scene, 1, dt).unwrap();
    let f100 = off.run(&gpu, &mut scene, 99, dt).unwrap();
    let f500 = off.run(&gpu, &mut scene, 400, dt).unwrap();

    for (name, f) in [("t0", &f0), ("t100", &f100), ("t500", &f500)] {
        let l = mean_luma(f);
        assert!(l > 2.0, "{name} looks blank (mean luma {l})");
        assert!(l < 200.0, "{name} looks blown out (mean luma {l})");
    }
    assert!(
        diff(&f0, &f100) > 0.002,
        "frame did not change between t0 and t100"
    );
    assert!(
        diff(&f100, &f500) > 0.01,
        "frame did not change between t100 and t500"
    );

    if let Ok(dir) = std::env::var("ROOKERY_SNAPSHOT_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, f) in [("t0", &f0), ("t100", &f100), ("t500", &f500)] {
            write_ppm(&dir.join(format!("rookery-{name}.ppm")), f);
        }
    }
}
