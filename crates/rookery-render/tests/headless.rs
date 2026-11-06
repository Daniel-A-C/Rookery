//! Headless smoke test: render the Living ASCII Rookery scene offscreen at several ticks and
//! check that frames are non-blank and evolve over time. Requires a GPU, so it is `#[ignore]`
//! by default. Run with: `cargo test -p rookery-render -- --ignored`.
//!
//! Set `ROOKERY_SNAPSHOT_DIR=/some/dir` to also dump a PPM preview.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rookery_core::world::WorldState;
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

#[test]
fn gif_and_png_encoding_smoke_test() {
    let tmp = tempfile::tempdir().unwrap();
    let png_path = tmp.path().join("test.png");
    let gif_path = tmp.path().join("test.gif");

    let f1 = [255u8, 0, 0, 255].repeat(16 * 16);
    let f2 = [0u8, 255, 0, 255].repeat(16 * 16);
    let mut frames = vec![f1.clone(), f2];

    rookery_render::capture::write_png(&png_path, 16, 16, &f1).unwrap();
    assert!(png_path.exists());
    assert!(std::fs::metadata(&png_path).unwrap().len() > 0);

    rookery_render::capture::write_gif(&gif_path, 16, 16, &mut frames, 10).unwrap();
    assert!(gif_path.exists());
    assert!(std::fs::metadata(&gif_path).unwrap().len() > 0);

    let mp4_path = tmp.path().join("test.mp4");
    let mp4_f1 = [255u8, 0, 0, 255].repeat(64 * 64);
    let mp4_f2 = [0u8, 255, 0, 255].repeat(64 * 64);
    let mp4_frames = vec![mp4_f1.clone(), mp4_f2.clone()];
    rookery_render::capture::write_mp4(&mp4_path, 64, 64, &mp4_frames, 30).unwrap();
    assert!(mp4_path.exists());
    assert!(std::fs::metadata(&mp4_path).unwrap().len() > 0);

    let stream_mp4_path = tmp.path().join("test_stream.mp4");
    let stream_frames = vec![Ok(mp4_f1), Ok(mp4_f2)];
    rookery_render::capture::encode_mp4_stream(&stream_mp4_path, 64, 64, 30, stream_frames)
        .unwrap();
    assert!(stream_mp4_path.exists());
    assert!(std::fs::metadata(&stream_mp4_path).unwrap().len() > 0);
}
