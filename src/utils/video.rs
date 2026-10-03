use std::path::Path;

use gstreamer as gst;
use gstreamer::prelude::*;
use gtk4::gdk;

pub struct VideoHandle {
    pub paintable: gdk::Paintable,
    pub pipeline: gst::Pipeline,
    pub playbin: gst::Element,
    pub rate: f64,
}

pub fn create_video(path: &Path) -> Option<VideoHandle> {
    gst::init().ok()?;
    gstgtk4::plugin_register_static().ok()?;

    let sink = gst::ElementFactory::make("gtk4paintablesink").build().ok()?;
    let playbin = gst::ElementFactory::make("playbin")
        .property("uri", format!("file://{}", path.display()))
        .property("video-sink", &sink)
        .build()
        .ok()?;

    let pipeline = gst::Pipeline::new();
    pipeline.add(&playbin).ok()?;

    let paintable = sink.property::<gdk::Paintable>("paintable");

    pipeline.set_state(gst::State::Playing).ok()?;

    Some(VideoHandle {
        paintable,
        pipeline,
        playbin,
        rate: 1.0,
    })
}

pub fn set_rate(handle: &mut VideoHandle, rate: f64) {
    if (handle.rate - rate).abs() < 0.01 {
        return;
    }
    handle.rate = rate;

    let _ = handle.playbin.seek(
        rate,
        gst::SeekFlags::INSTANT_RATE_CHANGE,
        gst::SeekType::None,
        gst::ClockTime::ZERO,
        gst::SeekType::None,
        gst::ClockTime::NONE,
    );
}

pub fn stop(handle: &VideoHandle) {
    let _ = handle.pipeline.set_state(gst::State::Null);
}
