use std::path::Path;

use gtk4::gdk;
use gtk4::glib;

pub fn load_gif_frames(path: &Path) -> Vec<(gdk::MemoryTexture, u32)> {
    let mut frames = Vec::new();

    if !path.exists() {
        log::error!("File not found: {}", path.display());
        return frames;
    }

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            log::error!("Cannot open {}: {}", path.display(), e);
            return frames;
        }
    };

    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);

    let mut decoder = match decoder.read_info(file) {
        Ok(d) => d,
        Err(e) => {
            log::error!("Cannot decode GIF {}: {}", path.display(), e);
            return frames;
        }
    };

    while let Ok(Some(frame)) = decoder.read_next_frame() {
        let width = frame.width as i32;
        let height = frame.height as i32;

        if frame.buffer.len() < (width * height * 4) as usize {
            continue;
        }

        let bytes = glib::Bytes::from(&frame.buffer);
        let texture = gdk::MemoryTexture::new(
            width,
            height,
            gdk::MemoryFormat::R8g8b8a8,
            &bytes,
            (width * 4) as usize,
        );

        let delay = if frame.delay == 0 { 100 } else { frame.delay as u32 * 10 };
        frames.push((texture, delay));
    }

    log::info!("Loaded {} frames from {}", frames.len(), path.display());
    frames
}
