use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::config::parser::Config;

pub fn create_overlay_window(app: &gtk4::Application, config: &Config) -> gtk4::Window {
    let window = gtk4::Window::builder()
        .application(app)
        .decorated(false)
        .build();

    window.add_css_class("anima-overlay");

    if gtk4_layer_shell::is_supported() {
        log::info!("wlr-layer-shell supported, using overlay mode.");
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_exclusive_zone(-1);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_margin(Edge::Top, config.position_y);
        window.set_margin(Edge::Left, config.position_x);
    } else {
        log::warn!("wlr-layer-shell not supported, falling back to regular window.");
        window.set_default_size(config.width, config.height);
    }

    window
}

pub fn make_draggable(window: &gtk4::Window, config: &Config) {
    let start = Rc::new(Cell::new((config.position_x, config.position_y)));

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
    gesture.connect_drag_update(move |_, offset_x, offset_y| {
        let (sx, sy) = start_update.get();
        window_update.set_margin(Edge::Left, sx + offset_x as i32);
        window_update.set_margin(Edge::Top, sy + offset_y as i32);
    });

    window.add_controller(gesture);
}
