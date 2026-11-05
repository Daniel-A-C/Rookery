//! Snapshot (PNG), loop (GIF), and high-framerate video (MP4) export utilities.

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

/// Encodes frames yielded by an iterator as an MP4 video (H.264 / AVC), streaming each frame
/// to disk with minimal memory footprint.
pub fn encode_mp4_stream<I, B>(
    path: &Path,
    width: u32,
    height: u32,
    fps: u32,
    frames: I,
) -> Result<()>
where
    I: IntoIterator<Item = Result<B>>,
    B: AsRef<[u8]>,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let mut encoder =
        openh264::encoder::Encoder::new().context("failed to initialize OpenH264 encoder")?;

    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let mut muxer = mp4e::Mp4e::new(&mut writer);
    muxer.set_video_track(width, height, mp4e::Codec::AVC);

    let safe_fps = fps.max(1) as u64;
    let mut frame_count = 0usize;

    for (idx, frame_res) in frames.into_iter().enumerate() {
        let rgba = frame_res.with_context(|| format!("failed to yield frame {idx}"))?;
        let rgba_source =
            openh264::formats::RgbaSliceU8::new(rgba.as_ref(), (width as usize, height as usize));
        let yuv = openh264::formats::YUVBuffer::from_rgb_source(rgba_source);
        let bitstream = encoder
            .encode(&yuv)
            .with_context(|| format!("failed to encode frame {idx}"))?;
        let bytes = bitstream.to_vec();
        if !bytes.is_empty() {
            let duration_ms = (((idx as u64 + 1) * 1000 / safe_fps)
                - (idx as u64 * 1000 / safe_fps))
                .max(1) as u32;
            muxer
                .encode_video(&bytes, duration_ms)
                .with_context(|| format!("failed to mux frame {idx} into MP4"))?;
        }
        frame_count += 1;
    }

    if frame_count == 0 {
        anyhow::bail!("no frames provided for MP4 encoding");
    }

    muxer.flush().context("failed to flush MP4 muxer")?;
    Ok(())
}

/// Writes multiple RGBA8 frames as an MP4 video (H.264 / AVC).
pub fn write_mp4(path: &Path, width: u32, height: u32, frames: &[Vec<u8>], fps: u32) -> Result<()> {
    encode_mp4_stream(path, width, height, fps, frames.iter().map(Ok))
}
