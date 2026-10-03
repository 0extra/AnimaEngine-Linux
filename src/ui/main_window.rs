use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, Box as GtkBox, Button, CheckButton, FileDialog, FileFilter, FlowBox,
    FlowBoxChild, Frame, Label, ListBox, ListBoxRow, Orientation, Paned, ProgressBar,
    Scale, ScrolledWindow, SearchEntry, SelectionMode, Separator,
};

use crate::config::parser::Config;
use crate::overlay::layer_surface;

pub struct LibraryItem {
    pub path: PathBuf,
    pub name: String,
    pub size: i32,
    pub speed: f64,
}

pub struct OverlayEntry {
    pub window: gtk4::Window,
    pub label: String,
    pub source: PathBuf,
}

pub struct AppState {
    pub library: Vec<LibraryItem>,
    pub overlays: Vec<OverlayEntry>,
    pub selected: Option<usize>,
}

pub fn show(app: &Application) {
    setup_css();

    let state = Rc::new(RefCell::new(AppState {
        library: Vec::new(),
        overlays: Vec::new(),
        selected: None,
    }));

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Anima-Linux")
        .default_width(1180)
        .default_height(720)
        .build();

    window.add_css_class("anima-main");

    let top_bar = GtkBox::new(Orientation::Horizontal, 8);
    top_bar.set_margin_top(10);
    top_bar.set_margin_bottom(10);
    top_bar.set_margin_start(12);
    top_bar.set_margin_end(12);

    let add_button = Button::with_label("Add from PC");
    add_button.add_css_class("anima-primary");
    let search_entry = SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search in your library..."));
    search_entry.set_hexpand(true);

    top_bar.append(&add_button);
    top_bar.append(&search_entry);

    let flow_box = FlowBox::new();
    flow_box.set_valign(Align::Start);
    flow_box.set_max_children_per_line(3);
    flow_box.set_min_children_per_line(2);
    flow_box.set_selection_mode(SelectionMode::Single);
    flow_box.set_homogeneous(true);
    flow_box.set_row_spacing(10);
    flow_box.set_column_spacing(10);
    flow_box.set_margin_top(10);
    flow_box.set_margin_bottom(10);
    flow_box.set_margin_start(10);
    flow_box.set_margin_end(10);

    let left_scroll = ScrolledWindow::builder()
        .child(&flow_box)
        .vexpand(true)
        .hexpand(true)
        .build();

    let left_box = GtkBox::new(Orientation::Vertical, 0);
    left_box.append(&top_bar);
    left_box.append(&left_scroll);

    let right_box = GtkBox::new(Orientation::Vertical, 10);
    right_box.set_margin_top(14);
    right_box.set_margin_bottom(14);
    right_box.set_margin_start(14);
    right_box.set_margin_end(14);
    right_box.set_width_request(320);

    // Контейнер под анимированное превью — пересобираем содержимое на каждый выбор.
    let preview_container = GtkBox::new(Orientation::Vertical, 0);
    preview_container.set_size_request(240, 240);
    preview_container.set_halign(Align::Center);
    preview_container.set_valign(Align::Center);

    let preview_frame = Frame::new(None);
    preview_frame.set_child(Some(&preview_container));
    preview_frame.set_size_request(240, 240);

    let selected_label = Label::new(Some("Selected animation"));
    selected_label.set_halign(Align::Start);
    selected_label.add_css_class("anima-title");

    let name_label = Label::new(Some("—"));
    name_label.set_halign(Align::Start);
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    name_label.set_max_width_chars(30);

    let size_row = GtkBox::new(Orientation::Horizontal, 8);
    size_row.append(&Label::new(Some("Size")));
    let size_scale = Scale::with_range(Orientation::Horizontal, 32.0, 800.0, 1.0);
    size_scale.set_value(300.0);
    size_scale.set_hexpand(true);
    size_scale.set_draw_value(true);
    size_scale.set_digits(0);
    size_row.append(&size_scale);

    let speed_row = GtkBox::new(Orientation::Horizontal, 8);
    speed_row.append(&Label::new(Some("Speed")));
    let speed_scale = Scale::with_range(Orientation::Horizontal, 0.25, 3.0, 0.05);
    speed_scale.set_value(1.0);
    speed_scale.set_hexpand(true);
    speed_scale.set_draw_value(true);
    speed_scale.set_digits(2);
    speed_row.append(&speed_scale);

    let remove_bg_check = CheckButton::with_label("Remove background (AI)");

    let launch_button = Button::with_label("Launch animation");
    launch_button.add_css_class("anima-primary");

    let remove_library_button = Button::with_label("Remove from library");
    remove_library_button.add_css_class("anima-danger");

    let overlays_label = Label::new(Some("Active overlays"));
    overlays_label.set_halign(Align::Start);
    overlays_label.add_css_class("anima-title");

    let overlays_list = ListBox::new();
    overlays_list.set_selection_mode(SelectionMode::None);
    let overlays_scroll = ScrolledWindow::builder()
        .min_content_height(120)
        .max_content_height(200)
        .child(&overlays_list)
        .build();

    let clear_overlays_button = Button::with_label("Close all overlays");
    clear_overlays_button.add_css_class("anima-danger");

    right_box.append(&preview_frame);
    right_box.append(&selected_label);
    right_box.append(&name_label);
    right_box.append(&size_row);
    right_box.append(&speed_row);
    right_box.append(&remove_bg_check);
    right_box.append(&Separator::new(Orientation::Horizontal));
    right_box.append(&launch_button);
    right_box.append(&remove_library_button);
    right_box.append(&Separator::new(Orientation::Horizontal));
    right_box.append(&overlays_label);
    right_box.append(&overlays_scroll);
    right_box.append(&clear_overlays_button);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&left_box));
    paned.set_end_child(Some(&right_box));
    paned.set_position(800);
    paned.set_resize_start_child(true);
    paned.set_shrink_start_child(false);
    paned.set_resize_end_child(false);
    paned.set_shrink_end_child(false);

    window.set_child(Some(&paned));

    {
        let state = state.clone();
        let flow_box = flow_box.clone();
        let parent = window.clone();
        add_button.connect_clicked(move |_| {
            let dialog = FileDialog::builder()
                .title("Choose a file")
                .modal(true)
                .build();

            let all_filter = FileFilter::new();
            all_filter.set_name(Some("All supported files"));
            for suffix in [
                "gif", "png", "jpg", "jpeg", "webp", "bmp", "tiff", "svg",
                "mp4", "webm", "mkv", "mov", "avi",
            ] {
                all_filter.add_suffix(suffix);
            }

            let image_filter = FileFilter::new();
            image_filter.set_name(Some("Images"));
            image_filter.add_mime_type("image/*");
            for suffix in ["gif", "webp", "png", "jpg", "jpeg", "bmp", "tiff", "svg"] {
                image_filter.add_suffix(suffix);
            }

            let video_filter = FileFilter::new();
            video_filter.set_name(Some("Videos"));
            for mime in ["video/mp4", "video/webm", "video/x-matroska", "video/quicktime"] {
                video_filter.add_mime_type(mime);
            }
            for suffix in ["mp4", "webm", "mkv", "mov", "avi"] {
                video_filter.add_suffix(suffix);
            }

            let filters = gio::ListStore::new::<FileFilter>();
            filters.append(&all_filter);
            filters.append(&image_filter);
            filters.append(&video_filter);
            dialog.set_filters(Some(&filters));
            dialog.set_default_filter(Some(&all_filter));

            let state = state.clone();
            let flow_box = flow_box.clone();
            dialog.open(Some(&parent), gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".into());
                        {
                            let mut s = state.borrow_mut();
                            s.library.push(LibraryItem {
                                path: path.clone(),
                                name: name.clone(),
                                size: 300,
                                speed: 1.0,
                            });
                        }
                        let child = make_library_child(&path, &name);
                        flow_box.append(&child);
                    }
                }
            });
        });
    }

    {
        let state = state.clone();
        let preview_container = preview_container.clone();
        let name_label = name_label.clone();
        let size_scale = size_scale.clone();
        let speed_scale = speed_scale.clone();
        flow_box.connect_child_activated(move |_, child| {
            let idx = child.index() as usize;
            let (path, name, size, speed) = {
                let s = state.borrow();
                match s.library.get(idx) {
                    Some(item) => (item.path.clone(), item.name.clone(), item.size, item.speed),
                    None => return,
                }
            };
            update_preview(&preview_container, Some(&path), size, speed);
            name_label.set_text(&name);
            size_scale.set_value(size as f64);
            speed_scale.set_value(speed);
            state.borrow_mut().selected = Some(idx);
        });
    }

    {
        let state = state.clone();
        let preview_container = preview_container.clone();
        size_scale.connect_value_changed(move |scale| {
            let v = scale.value() as i32;
            let (path, speed) = {
                let mut s = state.borrow_mut();
                match s.selected {
                    Some(idx) => match s.library.get_mut(idx) {
                        Some(item) => {
                            item.size = v;
                            (item.path.clone(), item.speed)
                        }
                        None => return,
                    },
                    None => return,
                }
            };
            update_preview(&preview_container, Some(&path), v, speed);
        });
    }

    {
        let state = state.clone();
        let preview_container = preview_container.clone();
        speed_scale.connect_value_changed(move |scale| {
            let v = scale.value();
            let (path, size) = {
                let mut s = state.borrow_mut();
                match s.selected {
                    Some(idx) => match s.library.get_mut(idx) {
                        Some(item) => {
                            item.speed = v;
                            (item.path.clone(), item.size)
                        }
                        None => return,
                    },
                    None => return,
                }
            };
            update_preview(&preview_container, Some(&path), size, v);
        });
    }

    {
        let state = state.clone();
        let window = window.clone();
        let overlays_list = overlays_list.clone();
        let remove_bg_check = remove_bg_check.clone();
        launch_button.connect_clicked(move |_| {
            let (path, size, name, speed) = {
                let s = state.borrow();
                match s.selected {
                    Some(idx) => match s.library.get(idx) {
                        Some(item) => (
                            item.path.clone(),
                            item.size,
                            item.name.clone(),
                            item.speed,
                        ),
                        None => return,
                    },
                    None => {
                        log::warn!("No animation selected");
                        return;
                    }
                }
            };

            let remove_bg_now = remove_bg_check.is_active();
            let path_str = path.to_string_lossy().to_string();

            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            let is_gif = ext == "gif";
            let is_video = matches!(ext.as_str(), "mp4" | "webm" | "mkv" | "mov" | "avi");

            if remove_bg_now && (is_gif || is_video) {
                let cache_dir = dirs::cache_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("anima-linux");
                let _ = std::fs::create_dir_all(&cache_dir);
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let processed = if is_gif {
                    cache_dir.join(format!("processed_{}.gif", stamp))
                } else {
                    cache_dir.join(format!("processed_{}.webm", stamp))
                };
                let processed_str = processed.to_string_lossy().to_string();

                let progress_dialog = gtk4::Window::builder()
                    .title("Removing background...")
                    .default_width(380)
                    .default_height(140)
                    .modal(true)
                    .transient_for(&window)
                    .build();
                progress_dialog.add_css_class("anima-main");

                let pbox = GtkBox::new(Orientation::Vertical, 10);
                pbox.set_margin_top(16);
                pbox.set_margin_bottom(16);
                pbox.set_margin_start(16);
                pbox.set_margin_end(16);

                let plabel = Label::new(Some("Preparing..."));
                let pbar = ProgressBar::new();
                pbar.set_show_text(true);
                pbar.set_fraction(0.0);

                pbox.append(&plabel);
                pbox.append(&pbar);
                progress_dialog.set_child(Some(&pbox));
                progress_dialog.present();

                let progress_state = Arc::new(Mutex::new((0usize, 0usize)));

                let ps_ui = progress_state.clone();
                let plabel_c = plabel.clone();
                let pbar_c = pbar.clone();
                glib::timeout_add_local(Duration::from_millis(150), move || {
                    let (cur, total) = *ps_ui.lock().unwrap();
                    if total > 0 {
                        let frac = cur as f64 / total as f64;
                        pbar_c.set_fraction(frac);
                        plabel_c.set_text(&format!("Frame {} / {}", cur, total));
                    } else if cur > 0 {
                        plabel_c.set_text(&format!("Decoded {} frames...", cur));
                    }
                    glib::ControlFlow::Continue
                });

                let (tx, rx) = mpsc::channel::<Result<(), String>>();
                let path_s = path_str.clone();
                let processed_s = processed_str.clone();
                let ps_thread = progress_state.clone();

                std::thread::spawn(move || {
                    let result = if is_gif {
                        crate::ai::gif_remover::process_gif(&path_s, &processed_s, |cur, total| {
                            let mut lock = ps_thread.lock().unwrap();
                            *lock = (cur, if total == 0 { cur } else { total });
                        })
                        .map_err(|e| e.to_string())
                    } else {
                        crate::ai::video_remover::process_video(
                            &path_s,
                            &processed_s,
                            |cur, total| {
                                let mut lock = ps_thread.lock().unwrap();
                                *lock = (cur, if total == 0 { cur } else { total });
                            },
                        )
                        .map_err(|e| e.to_string())
                    };
                    let _ = tx.send(result);
                });

                let rx = Rc::new(RefCell::new(rx));
                let state_c = state.clone();
                let list_c = overlays_list.clone();
                let win_c = window.clone();
                let dialog_c = progress_dialog.clone();
                let name_c = name.clone();
                let path_c = path.clone();

                glib::timeout_add_local(Duration::from_millis(200), move || {
                    let done = match rx.borrow().try_recv() {
                        Ok(res) => Some(res),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => {
                            Some(Err("Worker thread died".into()))
                        }
                    };

                    if let Some(res) = done {
                        dialog_c.close();
                        let final_path = match res {
                            Ok(()) => {
                                log::info!("Background removed: {}", processed_str);
                                processed_str.clone()
                            }
                            Err(e) => {
                                log::error!("Background removal failed: {}", e);
                                path_str.clone()
                            }
                        };
                        spawn_overlay(
                            &win_c,
                            &state_c,
                            &list_c,
                            &final_path,
                            &name_c,
                            size,
                            speed,
                            path_c.clone(),
                        );
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });

                return;
            }

            let actual_path = if remove_bg_now {
                let cache_dir = dirs::cache_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("anima-linux");
                let _ = std::fs::create_dir_all(&cache_dir);
                let processed = cache_dir.join(format!(
                    "processed_{}.png",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                ));
                match crate::ai::remover::process_image(
                    &path_str,
                    &processed.to_string_lossy(),
                    true,
                ) {
                    Ok(_) => processed.to_string_lossy().to_string(),
                    Err(e) => {
                        log::error!("Background removal failed: {}", e);
                        path_str.clone()
                    }
                }
            } else {
                path_str.clone()
            };

            spawn_overlay(
                &window,
                &state,
                &overlays_list,
                &actual_path,
                &name,
                size,
                speed,
                path.clone(),
            );
        });
    }

    {
        let state = state.clone();
        let flow_box = flow_box.clone();
        let preview_container = preview_container.clone();
        let name_label = name_label.clone();
        let overlays_list = overlays_list.clone();
        remove_library_button.connect_clicked(move |_| {
            let idx = match state.borrow().selected {
                Some(i) => i,
                None => return,
            };

            let removed_path = {
                let mut s = state.borrow_mut();
                if idx >= s.library.len() {
                    return;
                }
                let removed = s.library.remove(idx);
                s.selected = None;

                let mut i = 0;
                while i < s.overlays.len() {
                    if s.overlays[i].source == removed.path {
                        let entry = s.overlays.remove(i);
                        entry.window.close();
                    } else {
                        i += 1;
                    }
                }
                removed.path
            };

            if let Some(child) = flow_box.child_at_index(idx as i32) {
                flow_box.remove(&child);
            }
            update_preview(&preview_container, None, 300, 1.0);
            name_label.set_text("—");
            log::info!("Removed from library: {}", removed_path.display());

            rebuild_overlays(&overlays_list, &state);
        });
    }

    {
        let state = state.clone();
        let overlays_list = overlays_list.clone();
        clear_overlays_button.connect_clicked(move |_| {
            {
                let mut s = state.borrow_mut();
                for entry in s.overlays.drain(..) {
                    entry.window.close();
                }
            }
            rebuild_overlays(&overlays_list, &state);
        });
    }

    window.present();
}

fn update_preview(
    container: &GtkBox,
    path: Option<&std::path::Path>,
    size: i32,
    speed: f64,
) {
    // Снести предыдущего ребёнка.
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let Some(path) = path else { return };

    // Клампим размер, чтобы влезло в панель 240x240.
    let preview_size = size.clamp(32, 220);
    let path_str = path.to_string_lossy().to_string();
    let widget = crate::overlay::renderer::create_image_widget(&path_str, preview_size, speed);
    widget.set_halign(Align::Center);
    widget.set_valign(Align::Center);
    container.append(&widget);
}

fn spawn_overlay(
    window: &gtk4::ApplicationWindow,
    state: &Rc<RefCell<AppState>>,
    overlays_list: &ListBox,
    path: &str,
    name: &str,
    size: i32,
    speed: f64,
    source: PathBuf,
) {
    let cfg = Config {
        file_path: path.to_string(),
        position_x: 100,
        position_y: 100,
        width: size,
        height: size,
        remove_bg: false,
    };

    let overlay_app = window.application().unwrap();
    let overlay_win = layer_surface::create_overlay_window(&overlay_app, &cfg);
    let widget = crate::overlay::renderer::create_image_widget(path, size, speed);
    overlay_win.set_child(Some(&widget));
    layer_surface::make_draggable(&overlay_win, &cfg);
    overlay_win.present();

    state.borrow_mut().overlays.push(OverlayEntry {
        window: overlay_win,
        label: name.to_string(),
        source,
    });

    rebuild_overlays(overlays_list, state);
}

fn rebuild_overlays(list: &ListBox, state: &Rc<RefCell<AppState>>) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }

    let entries: Vec<(usize, String)> = state
        .borrow()
        .overlays
        .iter()
        .enumerate()
        .map(|(i, e)| (i, e.label.clone()))
        .collect();

    for (idx, label) in entries {
        let row = ListBoxRow::new();
        let hbox = GtkBox::new(Orientation::Horizontal, 6);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);
        hbox.set_margin_start(6);
        hbox.set_margin_end(6);

        let name = Label::new(Some(&format!("{}. {}", idx + 1, label)));
        name.set_hexpand(true);
        name.set_halign(Align::Start);
        name.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        hbox.append(&name);

        let close_btn = Button::with_label("Close");
        close_btn.add_css_class("anima-danger");
        let state_c = state.clone();
        let list_c = list.clone();
        close_btn.connect_clicked(move |_| {
            {
                let mut s = state_c.borrow_mut();
                if idx < s.overlays.len() {
                    let entry = s.overlays.remove(idx);
                    entry.window.close();
                }
            }
            rebuild_overlays(&list_c, &state_c);
        });
        hbox.append(&close_btn);

        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

fn setup_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(
        "
        window.anima-main {
            background-color: #1a1a1a;
            color: #e6e6e6;
        }
        window.anima-overlay {
            background-color: transparent;
            background-image: none;
        }
        window.anima-overlay > * {
            background-color: transparent;
            background-image: none;
        }
        .anima-title {
            font-size: 1.05em;
            font-weight: 600;
            color: #cfcfcf;
        }
        button {
            background-image: none;
            background-color: #2d2d2d;
            color: #e6e6e6;
            border: 1px solid #3a3a3a;
            border-radius: 6px;
            padding: 6px 12px;
            box-shadow: none;
            text-shadow: none;
        }
        button:hover {
            background-color: #3a3a3a;
        }
        button.anima-primary {
            background-color: #5b3d99;
            color: #ffffff;
            border: 1px solid #7c5cbf;
            font-weight: 600;
        }
        button.anima-primary:hover {
            background-color: #6b4da9;
        }
        button.anima-primary:active {
            background-color: #4b2d89;
        }
        button.anima-danger {
            background-color: #6b2e2e;
            color: #ffdddd;
            border: 1px solid #8b3a3a;
        }
        button.anima-danger:hover {
            background-color: #7b3e3e;
        }
        flowboxchild {
            background-color: #252525;
            border-radius: 8px;
            padding: 8px;
            border: 1px solid #2f2f2f;
        }
        flowboxchild:hover {
            background-color: #2d2d2d;
            border-color: #4a4a4a;
        }
        flowboxchild:selected {
            border-color: #7c5cbf;
            background-color: #2a2333;
        }
        frame {
            background-color: #1a1a1a;
            border: 1px solid #2f2f2f;
            border-radius: 8px;
        }
        scale trough {
            background-color: #2d2d2d;
            border-radius: 6px;
        }
        scale highlight {
            background-color: #7c5cbf;
        }
        progressbar trough {
            background-color: #2d2d2d;
            border-radius: 6px;
            min-height: 8px;
        }
        progressbar progress {
            background-color: #7c5cbf;
            border-radius: 6px;
            min-height: 8px;
        }
        entry, searchentry {
            background-color: #2d2d2d;
            color: #e6e6e6;
            border-radius: 6px;
            padding: 4px 8px;
            border: 1px solid #3a3a3a;
        }
        listbox {
            background-color: #202020;
            border-radius: 6px;
        }
        listboxrow {
            background-color: #252525;
            border-bottom: 1px solid #2f2f2f;
        }
        listboxrow:hover {
            background-color: #2d2d2d;
        }
        separator {
            background-color: #2f2f2f;
        }
        checkbutton {
            color: #e6e6e6;
        }
        .dim-label {
            opacity: 0.6;
        }
        ",
    );
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn make_library_child(path: &std::path::Path, name: &str) -> FlowBoxChild {
    let vbox = GtkBox::new(Orientation::Vertical, 6);

    let picture = gtk4::Picture::new();
    picture.set_content_fit(gtk4::ContentFit::Contain);
    picture.set_can_shrink(true);
    picture.set_size_request(140, 140);

    if let Some(texture) = crate::utils::thumbnail::generate(path) {
        picture.set_paintable(Some(&texture));
    } else {
        picture.set_filename(Some(path));
    }

    let label = Label::new(Some(name));
    label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    label.set_max_width_chars(18);
    label.set_halign(Align::Center);

    vbox.append(&picture);
    vbox.append(&label);

    let child = FlowBoxChild::new();
    child.set_child(Some(&vbox));
    child
}