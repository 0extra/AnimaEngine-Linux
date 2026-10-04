use std::path::Path;
use std::sync::OnceLock;

use gstreamer as gst;
use gstreamer::prelude::*;
use gtk4::glib;
use gtk4::gdk;

static PLUGIN_INIT: OnceLock<bool> = OnceLock::new();

fn ensure_plugin() -> bool {
    *PLUGIN_INIT.get_or_init(|| {
        if gst::init().is_err() {
            log::error!("gst::init failed");
            return false;
        }
        match gstgtk4::plugin_register_static() {
            Ok(_) => {
                log::info!("gstgtk4 plugin registered");
                true
            }
            Err(e) => {
                log::warn!("gstgtk4 register failed: {}", e);
                true
            }
        }
    })
}

pub struct VideoHandle {
    pub paintable: gdk::Paintable,
    pub pipeline: gst::Pipeline,
    pub playbin: gst::Element,
    pub rate: f64,
}

pub fn create_video(path: &Path) -> Option<VideoHandle> {
    if !ensure_plugin() {
        return None;
    }

    if gst::ElementFactory::find("gtk4paintablesink").is_none() {
        log::warn!("gtk4paintablesink element not available");
        return None;
    }

    let abs = path.canonicalize().ok()?;
    let uri = gst::glib::filename_to_uri(&abs, None).ok()?;

    let sink = gst::ElementFactory::make("gtk4paintablesink").build().ok()?;
    let playbin = gst::ElementFactory::make("playbin")
        .property("uri", uri.as_str())
        .property("video-sink", &sink)
        .build()
        .ok()?;

    let pipeline = gst::Pipeline::new();
    pipeline.add(&playbin).ok()?;

    let paintable = sink.property::<gdk::Paintable>("paintable");

    {
        let pipeline_weak = pipeline.downgrade();
        if let Some(bus) = pipeline.bus() {
            let _ = bus.add_watch_local(move |_, msg| {
                use gst::MessageView;
                if let MessageView::Eos(..) = msg.view() {
                    if let Some(p) = pipeline_weak.upgrade() {
                        log::info!("EOS received, seeking to 0");
                        let _ = p.seek_simple(
                            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                            gst::ClockTime::ZERO,
                        );
                    }
                }
                glib::ControlFlow::Continue
            });
        }
    }

    {
        let pipeline_weak = pipeline.downgrade();
        glib::timeout_add_local(std::time::Duration::from_millis(300), move || {
            let Some(p) = pipeline_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let pos = p.query_position::<gst::ClockTime>();
            let dur = p.query_duration::<gst::ClockTime>();
            if let (Some(pos), Some(dur)) = (pos, dur) {
                if dur > gst::ClockTime::ZERO
                    && pos + gst::ClockTime::from_mseconds(300) >= dur
                {
                    let _ = p.seek_simple(
                        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                        gst::ClockTime::ZERO,
                    );
                }
            }
            glib::ControlFlow::Continue
        });
    }

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