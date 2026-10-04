mod ai;
mod app;
mod config;
mod overlay;
mod ui;
mod utils;

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;

use gtk4::prelude::*;
use gtk4::Application;

fn acquire_instance_lock() -> Option<std::fs::File> {
    let dir = dirs::config_dir()?.join("anima-linux");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("instance.lock");

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .ok()?;

    let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        return None;
    }

    Some(file)
}

fn bootstrap_onnx_runtime() {
    if std::env::var("ORT_DYLIB_PATH").is_ok() {
        return;
    }

    let candidates = [
        "/usr/lib/libonnxruntime.so",
        "/usr/lib64/libonnxruntime.so",
        "/usr/lib/x86_64-linux-gnu/libonnxruntime.so",
        "/usr/lib/aarch64-linux-gnu/libonnxruntime.so",
        "/usr/local/lib/libonnxruntime.so",
        "/opt/onnxruntime/lib/libonnxruntime.so",
    ];
    for c in candidates {
        if std::path::Path::new(c).exists() {
            std::env::set_var("ORT_DYLIB_PATH", c);
            log::info!("ORT_DYLIB_PATH = {}", c);
            return;
        }
    }

    if let Ok(out) = std::process::Command::new("ldconfig").arg("-p").output() {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            if let Some(idx) = line.find("libonnxruntime.so") {
                if let Some(arrow) = line[idx..].find("=>") {
                    let p = line[idx + arrow + 2..].trim();
                    if std::path::Path::new(p).exists() {
                        std::env::set_var("ORT_DYLIB_PATH", p);
                        log::info!("ORT_DYLIB_PATH (ldconfig) = {}", p);
                        return;
                    }
                }
            }
        }
    }

    log::warn!("libonnxruntime.so not found; set ORT_DYLIB_PATH manually.");
}

fn detect_desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase()
}

fn is_gnome_wayland() -> bool {
    let d = detect_desktop();
    let is_gnome = d.contains("gnome") || d.contains("ubuntu");
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    is_gnome && is_wayland
}

fn force_backend_if_needed() {
    if std::env::var("GDK_BACKEND").is_ok() {
        log::info!(
            "GDK_BACKEND already set to {}",
            std::env::var("GDK_BACKEND").unwrap()
        );
        return;
    }

    if is_gnome_wayland() {
        log::info!("GNOME Wayland detected, forcing GDK_BACKEND=x11 (XWayland)");
        std::env::set_var("GDK_BACKEND", "x11");
    } else {
        log::info!("No backend override needed");
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let _lock = match acquire_instance_lock() {
        Some(l) => l,
        None => {
            log::warn!(
                "Another anima-linux instance is already running (or lock file busy). Exiting."
            );
            return;
        }
    };

    force_backend_if_needed();
    bootstrap_onnx_runtime();

    let cfg = match config::parser::Config::load("config.toml") {
        Ok(cfg) => {
            log::info!("Loaded config.toml");
            cfg
        }
        Err(_) => {
            log::debug!("config.toml not found, using defaults");
            config::parser::Config::default()
        }
    };

    ai::remover::configure(
        &cfg.ai_model,
        cfg.ai_input_size,
        cfg.models_dir.as_deref(),
    );

    let application = Application::builder()
        .application_id("com.github.anima-linux")
        .build();

    application.connect_activate(|app| {
        app::build_ui(app);
    });

    application.run();
}