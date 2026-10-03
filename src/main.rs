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
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let _cfg = match config::parser::Config::load("config.toml") {
        Ok(cfg) => {
            log::info!("Loaded config.toml");
            cfg
        }
        Err(_) => {
            log::debug!("config.toml not found, using defaults");
            config::parser::Config::default()
        }
    };

    let application = Application::builder()
        .application_id("com.github.anima-linux")
        .build();

    application.connect_activate(app::build_ui);
    application.run();
}
