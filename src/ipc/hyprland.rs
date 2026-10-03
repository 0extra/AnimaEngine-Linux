use std::env;

pub fn is_hyprland() -> bool {
    env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
}

pub fn log_status() {
    if is_hyprland() {
        log::info!("Обнаружен Hyprland. Интеграция IPC активна.");
        // Здесь в будущем можно добавить подписку на события сокета Hyprland
    }
}