use std::path::Path;

use anyhow::{anyhow, Result};
use image::RgbaImage;
use webp_animation::prelude::*;

use crate::ai::remover::remove_bg_image;

pub fn is_animated_webp(path: &Path) -> bool {
    let Ok(buf) = std::fs::read(path) else { return false };
    let Ok(decoder) = Decoder::new(&buf) else { return false };
    decoder.into_iter().take(2).count() > 1
}

pub fn process_animated_webp<F>(input: &str, output: &str, mut progress: F) -> Result<()>
where
    F: FnMut(usize, usize),
{
    let buffer = std::fs::read(input)?;
    let decoder = Decoder::new(&buffer)?;

    let mut frames: Vec<(Vec<u8>, i32)> = Vec::new();
    let mut prev_ts: i64 = 0;
    let mut dims: Option<(u32, u32)> = None;

    for frame in decoder.into_iter() {
        let (w, h) = frame.dimensions();
        dims = Some((w, h));
        let data = frame.data();
        if data.len() < (w * h * 4) as usize {
            continue;
        }

        let rgba = RgbaImage::from_raw(w, h, data.to_vec())
            .ok_or_else(|| anyhow!("Bad webp frame buffer"))?;
        let out = remove_bg_image(&rgba)?;

        let ts = frame.timestamp() as i64;
        let delay = (ts - prev_ts).max(10) as i32;
        prev_ts = ts;

        frames.push((out.into_raw(), delay));
        progress(frames.len(), 0);
    }

    if frames.is_empty() {
        return Err(anyhow!("Empty WebP or no frames decoded"));
    }

    let (w, h) = dims.ok_or_else(|| anyhow!("Unknown WebP dimensions"))?;

    let mut encoder = Encoder::new((w, h))?;
    let mut t_ms: i32 = 0;
    let total = frames.len();

    for (i, (raw, delay)) in frames.iter().enumerate() {
        encoder.add_frame(raw, t_ms)?;
        t_ms += *delay;
        progress(i + 1, total);
    }

    let data = encoder.finalize(t_ms)?;
    std::fs::write(output, data)?;
    Ok(())
}