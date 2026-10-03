mod ai;
mod app;
mod config;
mod ipc;
mod overlay;
mod ui;
mod utils;

use gtk4::prelude::*;
use gtk4::Application;

fn main() {
    env_logger::init();

    match config::parser::Config::load("config.toml") {
        Ok(cfg) => log::info!("Loaded config.toml: {:?}", cfg),
        Err(e) => log::warn!("config.toml not loaded: {}", e),
    }

    let application = Application::builder()
        .application_id("com.github.anima-linux")
        .build();

    application.connect_activate(app::build_ui);
    application.run();
}