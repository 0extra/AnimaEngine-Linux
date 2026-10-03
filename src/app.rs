pub fn build_ui(app: &gtk4::Application) {
    crate::ipc::hyprland::log_status();
    crate::ipc::driftwm::log_status();
    crate::ui::main_window::show(app);
}