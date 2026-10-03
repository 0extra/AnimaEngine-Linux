use std::path::Path;

use gtk4::gdk;
use gtk4::glib;
use webp_animation::prelude::*;

pub fn load_webp_frames(path: &Path) -> Vec<(gdk::MemoryTexture, u32)> {
    let mut frames = Vec::new();

    if !path.exists() {
        log::error!("File not found: {}", path.display());
        return frames;
    }

    let buffer = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            log::error!("Cannot read {}: {}", path.display(), e);
            return frames;
        }
    };

    let decoder = match Decoder::new(&buffer) {
        Ok(d) => d,
        Err(e) => {
            log::error!("Cannot decode WebP {}: {}", path.display(), e);
            return frames;
        }
    };

    let mut prev_ts: i64 = 0;

    for frame in decoder.into_iter() {
        let (w, h) = frame.dimensions();
        let data = frame.data();
        if data.len() < (w * h * 4) as usize {
            continue;
        }

        let width = w as i32;
        let height = h as i32;
        let bytes = glib::Bytes::from(data);
        let texture = gdk::MemoryTexture::new(
            width,
            height,
            gdk::MemoryFormat::R8g8b8a8,
            &bytes,
            (width * 4) as usize,
        );

        let ts = frame.timestamp() as i64;
        let delay_ms = (ts - prev_ts).max(10) as u32;
        prev_ts = ts;

        frames.push((texture, delay_ms));
    }

    log::info!("Loaded {} frames from {}", frames.len(), path.display());
    frames
}
