use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Image;

use crate::utils::gif;
use crate::utils::video;
use crate::utils::webp;

pub fn create_image_widget(file_path: &str, size: i32, speed: f64) -> gtk4::Widget {
    let path = Path::new(file_path);

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if matches!(ext.as_str(), "mp4" | "webm" | "mkv" | "mov" | "avi") {
        if let Some(mut handle) = video::create_video(path) {
            let picture = gtk4::Picture::for_paintable(&handle.paintable);
            picture.set_content_fit(gtk4::ContentFit::Contain);
            picture.set_can_shrink(true);
            picture.set_size_request(size, size);

            let fixed = gtk4::Fixed::new();
            fixed.set_size_request(size, size);
            fixed.set_overflow(gtk4::Overflow::Hidden);
            fixed.set_halign(gtk4::Align::Center);
            fixed.set_valign(gtk4::Align::Center);
            fixed.put(&picture, 0.0, 0.0);

            if (speed - 1.0).abs() > 0.01 {
                video::set_rate(&mut handle, speed);
            }

            let handle_rc = Rc::new(handle);
            fixed.connect_unrealize(move |_| {
                video::stop(&handle_rc);
            });
            return fixed.upcast();
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
    frames: Rc<Vec<(gtk4::gdk::MemoryTexture, u32)>>,
    current: Rc<RefCell<usize>>,
    size: i32,
    speed: f64,
    source_holder: Rc<RefCell<Option<glib::SourceId>>>,
) {
    // Widget was unrealized — stop the loop.
    if source_holder.borrow().is_none() && !image.is_realized() {
        // Only bail if we've been explicitly cancelled (unrealize handler fired).
        // is_realized() is false before first present, so we check both.
    }

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
            // If unrealize fired, holder is None and we stop.
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