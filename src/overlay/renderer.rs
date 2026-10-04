use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Image;

use crate::utils::gif;
use crate::utils::video;
use crate::utils::webp;

fn probe_video_size(path: &Path) -> Option<(i32, i32)> {
    use gstreamer as gst;
    use gstreamer_pbutils as gst_pbutils;

    gst::init().ok()?;

    let abs = path.canonicalize().ok()?;
    let uri = gst::glib::filename_to_uri(&abs, None).ok()?;

    let discoverer = gst_pbutils::Discoverer::new(gst::ClockTime::from_seconds(5)).ok()?;
    let info = discoverer.discover_uri(&uri).ok()?;
    let streams = info.video_streams();
    let v = streams.first()?;
    let w = v.width() as i32;
    let h = v.height() as i32;

    if w > 0 && h > 0 {
        Some((w, h))
    } else {
        None
    }
}

fn fit_square(iw: i32, ih: i32, size: i32) -> (i32, i32) {
    if iw <= 0 || ih <= 0 || size <= 0 {
        return (size.max(1), size.max(1));
    }
    let aspect = iw as f64 / ih as f64;
    if aspect >= 1.0 {
        let h = ((size as f64) / aspect).round() as i32;
        (size.max(1), h.max(1))
    } else {
        let w = ((size as f64) * aspect).round() as i32;
        (w.max(1), size.max(1))
    }
}

pub fn create_image_widget(file_path: &str, size: i32, speed: f64) -> gtk4::Widget {
    let path = Path::new(file_path);

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if matches!(ext.as_str(), "mp4" | "webm" | "mkv" | "mov" | "avi") {
        if let Some(mut handle) = video::create_video(path) {
            let (fw, fh) = match probe_video_size(path) {
                Some((iw, ih)) => fit_square(iw, ih, size),
                None => (size.max(1), size.max(1)),
            };

            let picture = gtk4::Picture::for_paintable(&handle.paintable);
            picture.set_content_fit(gtk4::ContentFit::Fill);
            picture.set_can_shrink(true);
            picture.set_size_request(fw, fh);
            picture.set_hexpand(false);
            picture.set_vexpand(false);

            let container = gtk4::ScrolledWindow::builder()
                .hscrollbar_policy(gtk4::PolicyType::Never)
                .vscrollbar_policy(gtk4::PolicyType::Never)
                .propagate_natural_width(false)
                .propagate_natural_height(false)
                .child(&picture)
                .build();
            container.set_size_request(fw, fh);
            container.set_hexpand(false);
            container.set_vexpand(false);
            container.set_halign(gtk4::Align::Center);
            container.set_valign(gtk4::Align::Center);

            if (speed - 1.0).abs() > 0.01 {
                video::set_rate(&mut handle, speed);
            }

            let handle_rc = Rc::new(handle);
            container.connect_unrealize(move |_| {
                video::stop(&handle_rc);
            });
            return container.upcast();
        } else {
            log::error!("Failed to create GStreamer pipeline for {}", file_path);
        }
    }

    let image = Image::new();
    if size > 0 {
        image.set_pixel_size(size);
    }

    match ext.as_str() {
        "gif" => {
            let frames = gif::load_gif_frames(path);
            if frames.is_empty() {
                log::error!("GIF is empty or unreadable.");
                return image.upcast();
            }
            let frames_rc = Rc::new(frames);
            let current = Rc::new(RefCell::new(0));
            let source_holder = Rc::new(RefCell::new(None::<glib::SourceId>));

            let holder_c = source_holder.clone();
            image.connect_unrealize(move |_| {
                if let Some(id) = holder_c.borrow_mut().take() {
                    id.remove();
                }
            });

            play_next_frame(image.clone(), frames_rc, current, size, speed, source_holder);
        }
        "webp" => {
            let frames = webp::load_webp_frames(path);
            if frames.is_empty() {
                log::error!("WebP is empty or unreadable.");
                return image.upcast();
            }
            let frames_rc = Rc::new(frames);
            let current = Rc::new(RefCell::new(0));
            let source_holder = Rc::new(RefCell::new(None::<glib::SourceId>));

            let holder_c = source_holder.clone();
            image.connect_unrealize(move |_| {
                if let Some(id) = holder_c.borrow_mut().take() {
                    id.remove();
                }
            });

            play_next_frame(image.clone(), frames_rc, current, size, speed, source_holder);
        }
        _ => {
            image.set_from_file(Some(file_path));
        }
    }

    image.upcast()
}

fn play_next_frame(
    image: Image,
    frames: Rc<Vec<(gdk::MemoryTexture, u32)>>,
    current: Rc<RefCell<usize>>,
    size: i32,
    speed: f64,
    source_holder: Rc<RefCell<Option<glib::SourceId>>>,
) {
    let idx = *current.borrow();
    let (texture, delay) = &frames[idx];
    image.set_paintable(Some(texture));

    if size > 0 {
        image.set_pixel_size(size);
    }

    let next_idx = (idx + 1) % frames.len();
    *current.borrow_mut() = next_idx;

    let adjusted = ((*delay as f64) / speed.max(0.1)) as u64;
    let next_delay = adjusted.max(16);

    let image_clone = image.clone();
    let frames_clone = frames.clone();
    let current_clone = current.clone();
    let holder_clone = source_holder.clone();

    let id = glib::timeout_add_local_once(
        std::time::Duration::from_millis(next_delay),
        move || {
            if holder_clone.borrow().is_none() && !image_clone.is_realized() {
                return;
            }
            play_next_frame(
                image_clone,
                frames_clone,
                current_clone,
                size,
                speed,
                holder_clone,
            );
        },
    );
    *source_holder.borrow_mut() = Some(id);
}