use std::path::Path;

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;

pub fn generate(path: &Path) -> Option<gdk::Texture> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "gif" => first_gif_frame(path),
        "webp" => first_webp_frame(path),
        "mp4" | "webm" | "mkv" | "mov" | "avi" => first_video_frame(path),
        _ => {
            let file = gtk4::gio::File::for_path(path);
            gdk::Texture::from_file(&file).ok()
        }
    }
}

fn first_gif_frame(path: &Path) -> Option<gdk::Texture> {
    let file = std::fs::File::open(path).ok()?;
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder.read_info(file).ok()?;
    let frame = decoder.read_next_frame().ok()??;

    let w = frame.width as i32;
    let h = frame.height as i32;
    if w <= 0 || h <= 0 {
        return None;
    }
    let expected = (w as usize) * (h as usize) * 4;
    if frame.buffer.len() < expected {
        return None;
    }

    let bytes = glib::Bytes::from(&frame.buffer[..expected]);
    let texture = gdk::MemoryTexture::new(
        w,
        h,
        gdk::MemoryFormat::R8g8b8a8,
        &bytes,
        (w * 4) as usize,
    );
    Some(texture.upcast())
}

fn first_webp_frame(path: &Path) -> Option<gdk::Texture> {
    use webp_animation::prelude::*;

    let buffer = std::fs::read(path).ok()?;
    let decoder = Decoder::new(&buffer).ok()?;
    let frame = decoder.into_iter().next()?;

    let (w, h) = frame.dimensions();
    let data = frame.data();
    let expected = (w as usize) * (h as usize) * 4;
    if w == 0 || h == 0 || data.len() < expected {
        return None;
    }

    let bytes = glib::Bytes::from(&data[..expected]);
    let texture = gdk::MemoryTexture::new(
        w as i32,
        h as i32,
        gdk::MemoryFormat::R8g8b8a8,
        &bytes,
        (w * 4) as usize,
    );
    Some(texture.upcast())
}

fn first_video_frame(path: &Path) -> Option<gdk::Texture> {
    use gstreamer as gst;
    use gstreamer::prelude::*;
    use gstreamer_app as gst_app;
    use gstreamer_video as gst_video;

    gst::init().ok()?;

    let abs = path.canonicalize().ok()?;
    let uri = gst::glib::filename_to_uri(&abs, None).ok()?;

    let desc = format!(
        "uridecodebin uri={} ! videoconvert ! video/x-raw,format=RGBA ! \
         appsink name=sink max-buffers=1 drop=true sync=false",
        uri.as_str()
    );

    let pipeline = gst::parse::launch(&desc)
        .ok()?
        .downcast::<gst::Pipeline>()
        .ok()?;

    let sink = pipeline
        .by_name("sink")?
        .downcast::<gst_app::AppSink>()
        .ok()?;

    pipeline.set_state(gst::State::Playing).ok()?;
    let sample = sink.try_pull_sample(gst::ClockTime::from_seconds(5));
    let _ = pipeline.set_state(gst::State::Null);

    let sample = sample?;
    let caps = sample.caps()?;
    let info = gst_video::VideoInfo::from_caps(caps).ok()?;
    let w = info.width() as i32;
    let h = info.height() as i32;
    if w <= 0 || h <= 0 {
        return None;
    }

    let buffer = sample.buffer()?;
    let map = buffer.map_readable().ok()?;
    let data = map.as_slice();
    let expected = (w as usize) * (h as usize) * 4;
    if data.len() < expected {
        return None;
    }

    let bytes = glib::Bytes::from(&data[..expected]);
    let texture = gdk::MemoryTexture::new(
        w,
        h,
        gdk::MemoryFormat::R8g8b8a8,
        &bytes,
        (w * 4) as usize,
    );
    Some(texture.upcast())
}