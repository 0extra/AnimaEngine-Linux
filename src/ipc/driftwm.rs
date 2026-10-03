use std::path::PathBuf;
use std::env;

pub fn is_driftwm() -> bool {
    // Проверяем наличие сокета DriftWM в XDG_RUNTIME_DIR
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        let socket_path = PathBuf::from(runtime_dir).join("driftwm.sock");
        return socket_path.exists();
    }
    false
}

pub fn log_status() {
    if is_driftwm() {
        log::info!("Обнаружен DriftWM. Помните, что для оверлея нужно правило 'widget = true'.");
    }
}