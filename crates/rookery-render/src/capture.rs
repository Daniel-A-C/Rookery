//! Snapshot (PNG) and loop (GIF) export utilities.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result};

/// Writes tightly packed RGBA8 pixels to a PNG file.
pub fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(&mut w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .context("failed to write PNG header")?;
    writer
        .write_image_data(rgba)
        .context("failed to write PNG image data")?;
    Ok(())
}

/// Writes multiple RGBA8 frames as an animated GIF loop.
pub fn write_gif(
    path: &Path,
    width: u16,
    height: u16,
    frames: &mut [Vec<u8>],
    delay_centisecs: u16,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut encoder =
        gif::Encoder::new(file, width, height, &[]).context("failed to initialize GIF encoder")?;
    encoder
        .set_repeat(gif::Repeat::Infinite)
        .context("failed to set GIF repeat")?;

    for (idx, rgba) in frames.iter_mut().enumerate() {
        let mut frame = gif::Frame::from_rgba_speed(width, height, rgba, 10);
        frame.delay = delay_centisecs;
        encoder
            .write_frame(&frame)
            .with_context(|| format!("failed to encode GIF frame {idx}"))?;
    }
    Ok(())
}
