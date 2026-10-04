use std::cell::Cell;
use std::rc::Rc;

use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::config::parser::Config;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayBackend {
    LayerShell,
    Toplevel,
}

pub fn detect_backend() -> OverlayBackend {
    let on_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let supported = gtk4_layer_shell::is_supported();
    if on_wayland && supported {
        OverlayBackend::LayerShell
    } else {
        OverlayBackend::Toplevel
    }
}

pub fn create_overlay_window(app: &gtk4::Application, config: &Config) -> gtk4::Window {
    let window = gtk4::Window::builder()
        .application(app)
        .decorated(false)
        .resizable(false)
        .build();

    window.add_css_class("anima-overlay");

    match detect_backend() {
        OverlayBackend::LayerShell => {
            log::info!("Overlay backend: layer-shell");
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_exclusive_zone(-1);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Left, true);
            window.set_margin(Edge::Top, config.position_y);
            window.set_margin(Edge::Left, config.position_x);
        }
        OverlayBackend::Toplevel => {
            log::info!("Overlay backend: toplevel (X11/XWayland)");
        }
    }

    window
}

pub fn make_draggable(
    widget: &gtk4::Widget,
    window: &gtk4::Window,
    config: &Config,
    position: Rc<Cell<(i32, i32)>>,
    on_change: Rc<dyn Fn()>,
) {
    position.set((config.position_x, config.position_y));
    match detect_backend() {
        OverlayBackend::LayerShell => {
            make_draggable_layer_shell(widget, window, position, on_change);
        }
        OverlayBackend::Toplevel => {
            make_draggable_toplevel(widget, window, on_change);
        }
    }
}

fn make_draggable_layer_shell(
    widget: &gtk4::Widget,
    window: &gtk4::Window,
    position: Rc<Cell<(i32, i32)>>,
    on_change: Rc<dyn Fn()>,
) {
    let start = Rc::new(Cell::new(position.get()));
    let gesture = gtk4::GestureDrag::new();

    let window_begin = window.clone();
    let start_begin = start.clone();
    gesture.connect_drag_begin(move |_, _, _| {
        start_begin.set((
            window_begin.margin(Edge::Left),
            window_begin.margin(Edge::Top),
        ));
    });

    let window_update = window.clone();
    let start_update = start.clone();
    let position_update = position.clone();
    gesture.connect_drag_update(move |_, offset_x, offset_y| {
        let (sx, sy) = start_update.get();
        let nx = sx + offset_x as i32;
        let ny = sy + offset_y as i32;
        window_update.set_margin(Edge::Left, nx);
        window_update.set_margin(Edge::Top, ny);
        position_update.set((nx, ny));
    });

    let on_change_end = on_change.clone();
    gesture.connect_drag_end(move |_, _, _| {
        on_change_end();
    });

    widget.add_controller(gesture);
}

fn make_draggable_toplevel(
    widget: &gtk4::Widget,
    window: &gtk4::Window,
    on_change: Rc<dyn Fn()>,
) {
    let gesture = gtk4::GestureDrag::new();

    let window_begin = window.clone();
    gesture.connect_drag_begin(move |gesture, start_x, start_y| {
        let Some(event) = gesture.current_event() else {
            return;
        };
        let Some(device) = event.device() else {
            return;
        };

        let (button, time) = if let Some(be) = event.downcast_ref::<gdk::ButtonEvent>() {
            (be.button(), be.time())
        } else {
            (1u32, 0u32)
        };

        let Some(surface) = window_begin.surface() else {
            return;
        };
        let Some(toplevel) = surface.downcast_ref::<gdk::Toplevel>() else {
            return;
        };

        toplevel.begin_move(&device, button as i32, start_x, start_y, time);
    });

    let on_change_end = on_change.clone();
    gesture.connect_drag_end(move |_, _, _| {
        on_change_end();
    });

    widget.add_controller(gesture);
}